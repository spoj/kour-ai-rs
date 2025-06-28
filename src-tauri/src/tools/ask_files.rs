use serde::{Deserialize};
use tokio::task;
use crate::error::Error;
use crate::utils::jailed::Jailed;
use futures::stream::{self, StreamExt};
use std::path::Path;
use serde_json::{json, Value};
use crate::tools::{Function, Tool};
use crate::chat::{ChatCompletionMessage, Content};

use crate::Result;

const MAX_CONCURRENCY: usize = 50;
const MAP_MODEL:&str = "google/gemini-2.5-flash";

#[derive(Deserialize)]
pub struct AskFilesArgs {
    pub query: String,
    pub filenames: Vec<String>,
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "ask_files".to_string(),
            description: "Answers a query about a list of files.".to_string(),
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
    let settings = task::spawn_blocking(crate::get_settings_fn)
        .await??;

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
                    ChatCompletionMessage::new("user", vec![Content::Text { text: format!("File: {filename}\n\nQuery: {query}") }])
                ];

                messages[0].content.extend(file_content);

                let response =
                    crate::chat::call_openrouter(&messages, model_name, "You are a helpful assistant that answers questions about files. Your answer must be grounded.", &vec![]).await?;
                let choice = &response.choices[0];
                let message: ChatCompletionMessage = choice.message.clone().into();
                if let Some(Content::Text { text }) = message.content.first() {
                    return Ok(json!({ filename: text, }));
                }
                
                Err(Error::Tool("MapError".to_string()))
            }
        })
        .buffer_unordered(MAX_CONCURRENCY)
        .collect()
        .await;
    
    Ok(responses)
}