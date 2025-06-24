use crate::tools;
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
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "modelName")]
    pub model_name: String,
    pub messages: Vec<ChatCompletionMessage>,
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
    messages: &Vec<ChatCompletionMessage>,
    api_key: &str,
    model_name: &str,
) -> super::Result<ChatCompletionResponse> {
    println!("Sending messages to OpenRouter: {:?}", messages);
    let client = reqwest::Client::new();
    let res = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": model_name,
            "messages": messages,
            "tools": &*tools::TOOLS,
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
    ToolCall(String),
    #[allow(dead_code)]
    ToolResult(String, String),
}

async fn execute_tool_call(
    tool_call: ToolCall,
    tx: mpsc::Sender<ChatUpdate>,
) -> super::Result<(String, String)> {
    tx.send(ChatUpdate::ToolCall(tool_call.function.name.clone()))
        .await?;
    let result = match tools::tool_executor(
        &tool_call.function.name,
        &tool_call.function.arguments,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => e.to_string(),
    };
    tx.send(ChatUpdate::ToolResult(
        tool_call.function.name.clone(),
        result.clone(),
    ))
    .await?;

    Ok((tool_call.id, result))
}

pub async fn handle_tool_calls(
    tool_calls: Vec<ToolCall>,
    tx: mpsc::Sender<ChatUpdate>,
) -> super::Result<Vec<ChatCompletionMessage>> {
    let mut new_messages = Vec::new();

    new_messages
        .push(ChatCompletionMessage::new("assistant", vec![]).tool_calls(tool_calls.clone()));

    let tool_futs = tool_calls
        .into_iter()
        .map(|tool_call| tokio::spawn(execute_tool_call(tool_call, tx.clone())));

    let tool_results = join_all(tool_futs).await;

    for tool_result in tool_results {
        let (id, result) = tool_result??;
        new_messages.push(ChatCompletionMessage {
            role: "tool".to_string(),
            content: vec![Content::Text { text: result }],
            tool_call_id: Some(id),
            tool_calls: None,
        });
    }

    Ok(new_messages)
}
