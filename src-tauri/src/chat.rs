use crate::tools;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatCompletionMessage {
    pub fn new(role: &str) -> Self {
        Self {
            role: role.to_string(),
            content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }

    pub fn tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

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
    pub message: ChatCompletionMessage,
}

pub async fn call_openrouter(
    messages: &Vec<ChatCompletionMessage>,
    api_key: &str,
    model_name: &str,
) -> super::Result<ChatCompletionResponse> {
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
        .await?
        .json::<ChatCompletionResponse>()
        .await?;
    Ok(res)
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
    let result =
        tools::tool_executor(&tool_call.function.name, &tool_call.function.arguments).await;
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

    new_messages.push(ChatCompletionMessage::new("assistant").tool_calls(tool_calls.clone()));

    let tool_futs = tool_calls
        .into_iter()
        .map(|tool_call| tokio::spawn(execute_tool_call(tool_call, tx.clone())));

    let tool_results = join_all(tool_futs).await;

    for tool_result in tool_results {
        let (id, result) = tool_result??;
        new_messages.push(
            ChatCompletionMessage::new("tool")
                .tool_call_id(&id)
                .content(&result),
        );
    }
    Ok(new_messages)
}
