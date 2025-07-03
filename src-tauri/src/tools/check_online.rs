use crate::Result;
use crate::chat::{OutgoingMessage, Content, call_openrouter};
use crate::error::Error;
use crate::tools::{Function, Tool};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_value};
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

    let messages = vec![OutgoingMessage::new(
        "user",
        vec![
            Content::Text {
                text: "Research user query on the internet. take the broader context in consideration. Give both answer and citations.".to_string(),
            },
            Content::Text {
                text: format!("Broader context:\n{}", args.broader_context),
            },
            Content::Text {
                text: format!("Query:\n{}", args.query),
            },
        ],
    )];
    let schema = to_value(schema_for!(CheckOnlineResult)).unwrap(); // unwrap: all input controlled by code
    let response = call_openrouter(&messages, SEARCH_MODEL, "", &vec![], Some(schema)).await?;

    let choice = &response.choices[0];
    let message: OutgoingMessage = choice.message.clone().into();
    if let Some(Content::Text { text }) = message.content.first() {
        return Ok(from_str(text)?);
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
            description: "Perform an internet search for facts using the Perplexity model."
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
