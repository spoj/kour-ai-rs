use crate::{get_settings_fn, settings::Settings, tools};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
    #[serde(rename = "file")]
    File { file: FileData },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileData {
    pub filename: String,
    pub file_data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
#[derive(Default)]
enum MessageContent {
    Text(String),
    Parts(Vec<Content>),
    #[default]
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingMessage {
    pub role: String,
    #[serde(default)]
    content: MessageContent,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl From<IncomingMessage> for ChatCompletionMessage {
    fn from(msg: IncomingMessage) -> Self {
        let content = match msg.content {
            MessageContent::Text(text) => vec![Content::Text { text }],
            MessageContent::Parts(parts) => parts,
            MessageContent::None => vec![],
        };

        ChatCompletionMessage {
            role: msg.role,
            content,
            tool_calls: msg.tool_calls,
            tool_call_id: None,
        }
    }
}

impl ChatCompletionMessage {
    pub fn new(role: &str, content: Vec<Content>) -> Self {
        Self {
            role: role.to_string(),
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self.content = vec![]; // Empty vec will be omitted by serde
        self
    }

    #[allow(dead_code)]
    pub fn tool_call_id(mut self, tool_call_id: &str) -> Self {
        self.tool_call_id = Some(tool_call_id.to_string());
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionOptions {
    #[serde(rename = "modelName")]
    pub model_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: IncomingMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponseDelta {
    pub choices: Vec<ChoiceDelta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChoiceDelta {
    pub delta: IncomingMessage,
}

pub async fn call_openrouter(
    messages: &[ChatCompletionMessage],
    model_name: &str,
    system_prompt: &str,
    tools: &Vec<tools::Tool>,
) -> super::Result<ChatCompletionResponse> {
    println!("Sending messages to OpenRouter: {:?}", messages);
    let settings = get_settings_fn()?;
    let client = reqwest::Client::new();
    let mut final_messages = messages.to_vec();
    if !system_prompt.is_empty() {
        final_messages.insert(
            0,
            ChatCompletionMessage::new(
                "system",
                vec![Content::Text {
                    text: system_prompt.to_string(),
                }],
            ),
        );
    }
    let res = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(&settings.api_key)
        .json(&serde_json::json!({
            "model": model_name,
            "messages": final_messages,
            "tools": tools,
            "provider": {
                "order": settings.provider_order.split(',').collect::<Vec<_>>(),
            }
        }))
        .send()
        .await?;

    let text = res.text().await?;
    println!("Got response from OpenRouter: {}", text);
    let response: ChatCompletionResponse =
        match serde_json::from_str::<ChatCompletionResponse>(&text) {
            Ok(res) => res,
            Err(_) => ChatCompletionResponse {
                choices: vec![Choice {
                    message: IncomingMessage {
                        role: "assistant".to_string(),
                        content: MessageContent::Text(text),
                        tool_calls: None,
                    },
                }],
            },
        };
    Ok(response)
}

#[derive(Debug, Clone)]
pub enum ChatUpdate {
    ToolCall {
        name: String,
        id: String,
        arguments: String,
    },
    ToolResult {
        id: String,
        result: String,
    },
}

async fn execute_tool_call(
    tool_call: ToolCall,
    tx: mpsc::Sender<ChatUpdate>,
) -> super::Result<(String, String)> {
    tx.send(ChatUpdate::ToolCall {
        name: tool_call.function.name.clone(),
        id: tool_call.id.clone(),
        arguments: tool_call.function.arguments.clone(),
    })
    .await?;
    let json_value =
        match tools::tool_executor(&tool_call.function.name, &tool_call.function.arguments).await {
            Ok(value) => value,
            Err(e) => serde_json::Value::String(e.to_string()),
        };

    let result = if tool_call.function.name == "load_file" {
        if let Some(mut obj) = json_value.as_object().cloned() {
            obj.remove("user_message");
            serde_json::to_string(&obj).unwrap_or_else(|_| json_value.to_string())
        } else {
            json_value.to_string()
        }
    } else {
        serde_json::to_string(&json_value).unwrap_or_else(|_| json_value.to_string())
    };
    tx.send(ChatUpdate::ToolResult {
        id: tool_call.id.clone(),
        result: result.clone(),
    })
    .await?;

    Ok((tool_call.id, result))
}

pub async fn handle_tool_calls(
    tool_calls: Vec<ToolCall>,
    tx: mpsc::Sender<ChatUpdate>,
) -> super::Result<Vec<ChatCompletionMessage>> {
    let mut new_messages = Vec::new();


    let tool_futs = tool_calls
        .into_iter()
        .map(|tool_call| tokio::spawn(execute_tool_call(tool_call, tx.clone())));

    let tool_results = join_all(tool_futs).await;

    for tool_result in tool_results {
        let (id, result_str) = tool_result??;

        // Try to deserialize the result into our special LoadFileResult structure
        if let Ok(file_result) = serde_json::from_str::<serde_json::Value>(&result_str) {
            if file_result.get("type").and_then(|t| t.as_str()) == Some("file_loaded") {
                let display_message = file_result["display_message"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                // The user_message is nested in the JSON, deserialize it separately
                if let Ok(user_message) = serde_json::from_value::<ChatCompletionMessage>(
                    file_result["user_message"].clone(),
                ) {
                    // 1. Add the simple tool message for display
                    new_messages.push(ChatCompletionMessage {
                        role: "tool".to_string(),
                        content: vec![Content::Text {
                            text: display_message,
                        }],
                        tool_call_id: Some(id),
                        tool_calls: None,
                    });

                    // 2. Add the rich user message with the actual file content
                    new_messages.push(user_message);

                    continue; // Skip the default handling below
                }
            }
        }

        // Default handling for all other tools
        new_messages.push(ChatCompletionMessage {
            role: "tool".to_string(),
            content: vec![Content::Text { text: result_str }],
            tool_call_id: Some(id),
            tool_calls: None,
        });
    }

    Ok(new_messages)
}
