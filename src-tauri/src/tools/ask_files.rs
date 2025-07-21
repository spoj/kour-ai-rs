use crate::error::Error;
use crate::openrouter::{IncomingContent, Openrouter};
use crate::search::{SEARCH_STATE, SELECTION_STATE, search_files_by_name};
use crate::tools::{Function, Tool};
use crate::utils::jailed::Jailed;
use futures::stream::{self, StreamExt};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, json, to_value};
use std::path::Path;
use tokio::task;

use crate::Result;

const MAX_CONCURRENCY: usize = 50;
const MAP_MODEL: &str = "google/gemini-2.5-flash";

#[derive(Deserialize)]
pub struct AskFilesArgs {
    pub query: String,
    pub filenames: Vec<String>,
}

#[derive(Deserialize)]
pub struct AskFilesGlobArgs {
    pub query: String,
    pub pattern: String,
    pub max_results: usize,
}

#[derive(Deserialize)]
pub struct AskFilesSearchedArgs {
    pub query: String,
    pub max_results: usize,
}
#[derive(Deserialize)]
pub struct UserSelectedArgs {
    pub max_results: usize,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AskFileResults {
    answer: String,
    extracts: Vec<String>,
}
pub fn ask_files_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "ask_files".to_string(),
            description: "Queries a specific, user-provided list of files in parallel, making it efficient for targeted analysis of known files. Expects to be given the query and a broader_context. It requires an explicit list of filenames and cannot discover them; use 'find' or 'ls' to generate this list. Works best for simple fact-finding queries.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query to run against each file."
                    },
                    "filenames": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "A list of filenames to run the query against."
                    }
                },
                "required": ["query", "filenames"]
            }),
        },
    }
}

pub async fn ask_files(args: AskFilesArgs) -> Result<Vec<Result<Value>>> {
    let AskFilesArgs { query, filenames } = args;
    let settings = crate::settings::get_settings()?;

    let responses: Vec<_> = stream::iter(filenames)
        .map(|filename| {
            let root_dir = settings.root_dir.clone();
            let query = query.clone();
            let model_name = MAP_MODEL;

            async move {
                let jail = Path::new(&root_dir);
                let file_path = jail.jailed_join(Path::new(&filename))?;
                let file_content =
                    task::spawn_blocking(move || crate::file_handler::process_file_for_llm(&file_path))
                        .await??;

                let mut messages = vec![
                    json!({"role":"user","content":format!("File: {filename}\n\nQuery: {query}")})
                ];

                messages.push(json!({"role":"user","content":file_content}));
                let schema = to_value(schema_for!(AskFileResults)).unwrap(); // unwrap: all input controlled by code

                let response =
                    Openrouter::call(&messages, model_name, "You are a helpful assistant that answers questions about files. Your answer must be grounded.", &vec![], Some(schema)).await?;
                if let IncomingContent::Text(text) =  &response.choices[0].message.content
                && let Ok(output) = from_str::<AskFileResults>(text)
                {
                    return Ok(json!({filename:output}));
                }

                Err(Error::Tool("MapError".to_string()))
            }
        })
        .buffer_unordered(MAX_CONCURRENCY)
        .collect()
        .await;

    Ok(responses)
}

pub fn ask_files_glob_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "ask_files_glob".to_string(),
            description: "Same as ask_files, but accepts a glob pattern to match more than 1 file. Must specify max_results".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query to run against each file."
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Pattern used to match files. Same logic as the `find` tool pattern"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum results. If glob matches more than this, the tool will return an error to avoid overwhelming the user. Start with 100 and adjust up if the task really requires understanding more files."
                    }
                },
                "required": ["query", "pattern","max_results"]
            }),
        },
    }
}

pub async fn ask_files_glob(args: AskFilesGlobArgs) -> Result<Vec<Result<Value>>> {
    let AskFilesGlobArgs {
        query,
        pattern,
        max_results,
    } = args;
    let filenames = search_files_by_name(&pattern)?;

    if filenames.len() > max_results {
        return Err(Error::Tool(format!(
            "Error: Found more files ({}) than limit ({}). Consider up the limit or take different approach.",
            max_results,
            filenames.len()
        )));
    }

    ask_files(AskFilesArgs { query, filenames }).await
}

pub fn ask_files_selected_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "ask_files_selected".to_string(),
            description: format!(
                "Same as ask_files, but applies directly to a set of user specified files in the App interface. User has selected {} items. If user has not selected anything actively, this tool will query all files returned by an active user search, which currently has {} items",
                SELECTION_STATE.selection.read().unwrap().len(),
                SEARCH_STATE.last_search_result.read().unwrap().len(),
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query to run against each file."
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum results. If user selected more than this, the tool will return an error to avoid overwhelming the user. Start with 100 and adjust up if the task really requires understanding more files."
                    }
                },
                "required": ["query", "max_results"]
            }),
        },
    }
}

pub async fn ask_files_selected(args: AskFilesSearchedArgs) -> Result<Vec<Result<Value>>> {
    let AskFilesSearchedArgs { query, max_results } = args;
    let filenames;
    {
        let selection = SELECTION_STATE.selection.read().unwrap();
        let result = SEARCH_STATE.last_search_result.read().unwrap();
        filenames = if !selection.is_empty() {
            selection.iter().cloned().collect()
        } else {
            result.to_vec()
        };
    }

    if filenames.len() > max_results {
        return Err(Error::Tool(format!(
            "Error: Found more files ({}) than limit ({}). Consider up the limit or take different approach.",
            max_results,
            filenames.len()
        )));
    }

    ask_files(AskFilesArgs { query, filenames }).await
}

pub fn list_user_selected_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "list_user_selected".to_string(),
            description: format!(
                "List the {} files that that user has selected, or the {} files that the user has in the search results if nothing is selected actively.",
                SELECTION_STATE.selection.read().unwrap().len(),
                SEARCH_STATE.last_search_result.read().unwrap().len(),
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "max_results": {
                        "type": "number",
                        "description": "Maximum results. If user selected more than this, the tool will return an error to avoid overwhelming the user. Start with 100 and adjust up if the task really requires understanding more files."
                    }
                },
                "required": ["max_results"]
            }),
        },
    }
}

pub async fn list_user_selected(args: UserSelectedArgs) -> Result<Vec<String>> {
    let UserSelectedArgs { max_results } = args;
    let filenames;
    {
        let selection = SELECTION_STATE.selection.read().unwrap();
        let result = SEARCH_STATE.last_search_result.read().unwrap();
        filenames = if !selection.is_empty() {
            selection.iter().cloned().collect()
        } else {
            result.to_vec()
        };
    }

    if filenames.len() > max_results {
        return Err(Error::Tool(format!(
            "Error: Found more files ({}) than limit ({}). Consider up the limit or take different approach.",
            max_results,
            filenames.len()
        )));
    }

    Ok(filenames)
}
