use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use serde_json::to_string_pretty;

use crate::get_settings_fn;
use crate::interaction::Content;
use crate::interaction::Interaction;
use crate::interaction::Source;
use crate::interaction::Target;
use crate::tools;

pub struct Openrouter;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingMessage {
    pub role: String,
    #[serde(default)]
    pub content: IncomingContent,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
#[derive(Default)]
pub enum IncomingContent {
    Text(String),
    Parts(Vec<Content>),
    #[default]
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: IncomingMessage,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatOptions {
    #[serde(rename = "modelName")]
    pub model_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

impl<'a> Target<'a> for Openrouter {
    type RenderType = Value;
    fn convert(interaction: &Interaction) -> Vec<Value> {
        match interaction {
            Interaction::LlmResponse {
                content,
                tool_calls,
            } => {
                if let Some(tool_calls) = tool_calls {
                    vec![json!({
                    "role": "assistant",
                    "tool_calls": tool_calls})]
                } else {
                    vec![json!({
                    "role": "assistant",
                    "content": content})]
                }
            }
            Interaction::ToolResult {
                tool_call_id,
                response,
                for_llm,
                ..
            } => {
                let mut out = vec![json!({
                "role": "tool",
                "content": response,
                "tool_call_id": tool_call_id,
                })];
                if !for_llm.is_empty() {
                    out.push(json!({
                    "role": "user",
                    "content": for_llm,
                    }));
                }
                out
            }
            Interaction::UserMessage { content } => vec![json!({
                    "role": "user",
                    "content": content,
            })],
        }
    }
}
impl Source for Openrouter {
    type SendType = IncomingMessage;

    fn sends(data: IncomingMessage) -> Interaction {
        let tool_calls = data.tool_calls;
        let content = match data.content {
            IncomingContent::Text(t) => vec![Content::Text { text: t }],
            IncomingContent::Parts(contents) => contents,
            IncomingContent::None => vec![],
        };
        Interaction::LlmResponse {
            content,
            tool_calls,
        }
    }
}

impl Openrouter {
    pub async fn call(
        messages: &[Value],
        model_name: &str,
        system_prompt: &str,
        tools: &Vec<tools::Tool>,
        schema: Option<Value>,
    ) -> super::Result<ChatResponse> {
        println!(
            "Sending messages to OpenRouter: {}",
            messages
                .iter()
                .map(|j| to_string_pretty(j).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n\n")
        );
        let settings = get_settings_fn()?;
        let client = reqwest::Client::new();
        let mut final_messages = messages.to_vec();
        if !system_prompt.is_empty() {
            final_messages.insert(0, json!({"role":"system","content":system_prompt}));
        }
        let mut json_to_send = json!({
            "model": model_name,
            "messages": final_messages,
            "tools": tools,
            "provider": {
                "order": settings.provider_order.split(',').collect::<Vec<_>>(),
            }
        });
        if let Some(schema) = schema
            && let Some(m) = json_to_send.as_object_mut()
        {
            m.insert(
                "response_format".to_string(),
                json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "output",
                        "strict": true,
                        "schema": schema
                    }
                }),
            );
        }
        let res = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&settings.api_key)
            .json(&json_to_send)
            .send()
            .await?;

        let text = res.text().await?;
        println!("Got response from OpenRouter: {text}",);
        let response: ChatResponse = match serde_json::from_str::<ChatResponse>(&text) {
            Ok(res) => res,
            Err(_) => ChatResponse {
                choices: vec![Choice {
                    message: IncomingMessage {
                        role: "assistant".to_string(),
                        content: IncomingContent::Text(text),
                        tool_calls: None,
                    },
                }],
            },
        };
        Ok(response)
    }
}
