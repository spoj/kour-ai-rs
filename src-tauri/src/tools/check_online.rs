use crate::chat::{ChatCompletionMessage, Content};
use crate::error::Error;
use crate::tools::{Function, Tool};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::task;

const SEARCH_MODEL: &str = "perplexity/sonar";

#[derive(Deserialize)]
pub struct CheckOnlineArgs {
    pub query: String,
    #[serde(default)]
    pub broader_context: String,
}

#[derive(Serialize)]
pub struct CheckOnlineResult {
    content: String,
    citations: serde_json::Value, // Annotations can be complex, so Value is flexible
}

pub async fn check_online(args: CheckOnlineArgs) -> Result<CheckOnlineResult> {
    let settings = task::spawn_blocking(crate::get_settings_fn)
        .await??;

    if settings.api_key.is_empty() {
        return Err(Error::Tool(
            "Error: OpenRouter API key is not set.".to_string(),
        ));
    }

    let messages = vec![ChatCompletionMessage::new(
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

    let response = crate::chat::call_openrouter(&messages, SEARCH_MODEL, "", &vec![]).await?;

    if let Some(choice) = response.choices.first() {
        let chat_message: ChatCompletionMessage = choice.message.clone().into();
        if let Some(Content::Text { text }) = chat_message.content.first() {
            // Assuming annotations are part of the response, though not typed in our current struct
            // We'll just pass an empty array for now. A more robust impl would parse this.
            let result = CheckOnlineResult {
                content: text.clone(),
                citations: json!([]),
            };
            return Ok(result);
        }
    }
    
    Err(Error::Tool("Failed to get a valid response from the online search tool.".to_string()))
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "check_online".to_string(),
            description: "Perform an internet search for facts using the Perplexity model.".to_string(),
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