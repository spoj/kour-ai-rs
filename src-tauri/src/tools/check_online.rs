use crate::Result;
use crate::error::Error;
use crate::interaction::Content;
use crate::openrouter::{IncomingContent, Openrouter};
use crate::tools::{Function, Tool};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_value};
use tokio::task;

const SEARCH_MODEL: &str = "perplexity/sonar";

#[derive(Deserialize)]
pub struct CheckOnlineArgs {
    pub query: String,
    #[serde(default)]
    pub broader_context: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CheckOnlineResult {
    content: String,
    citations: Vec<String>,
}

pub async fn check_online(args: CheckOnlineArgs) -> Result<CheckOnlineResult> {
    let settings = task::spawn_blocking(crate::get_settings_fn).await??;

    if settings.api_key.is_empty() {
        return Err(Error::Tool(
            "Error: OpenRouter API key is not set.".to_string(),
        ));
    }

    let message = json!({
    "role":"user",
    "content":vec![
        Content::Text {
            text: "Research user query on the internet. take the broader context in consideration. Give both answer and citations.".to_string(),
        },
        Content::Text {
            text: format!("Broader context:\n{}", args.broader_context),
        },
        Content::Text {
            text: format!("Query:\n{}", args.query),
        },
    ]
    });
    let schema = to_value(schema_for!(CheckOnlineResult)).unwrap(); // unwrap: all input controlled by code
    let response = Openrouter::call(&[message], SEARCH_MODEL, "", &vec![], Some(schema)).await?;
    if let IncomingContent::Text(text) = &response.choices[0].message.content {
        let result: CheckOnlineResult = from_str(text)?;
        return Ok(result);
    }
    Err(Error::Tool(
        "Failed to get a valid response from the online search tool.".to_string(),
    ))
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "check_online".to_string(),
            description: "Performs an internet search using Perplexity Sonar to find facts and answer queries. It's best for getting up-to-date information or answers to general knowledge questions."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query to search for."
                    },
                    "broader_context": {
                        "type": "string",
                        "description": "Optional broader context for the query."
                    }
                },
                "required": ["query"]
            }),
        },
    }
}
