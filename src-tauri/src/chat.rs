use futures::future::join_all;
use serde::{Deserialize, Serialize};
use crate::tools;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IChatCompletionMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IChatCompletionOptions {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "modelName")]
    pub model_name: String,
    pub messages: Vec<IChatCompletionMessage>,
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
    pub message: IChatCompletionMessage,
}

pub async fn call_openrouter(
    messages: &Vec<IChatCompletionMessage>,
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
    ToolResult(String),
}

pub async fn handle_tool_calls(
    messages: &mut Vec<IChatCompletionMessage>,
    tool_calls: Vec<ToolCall>,
    tx: tokio::sync::mpsc::Sender<ChatUpdate>,
) {
    messages.push(IChatCompletionMessage {
        role: "assistant".to_string(),
        content: None,
        tool_calls: Some(tool_calls.clone()),
        tool_call_id: None,
    });

    let tool_futs = tool_calls.into_iter().map(|tool_call| {
        let tx = tx.clone();
        tokio::spawn(async move {
            tx.send(ChatUpdate::ToolCall(tool_call.function.name.clone()))
                .await
                .unwrap();
            let result =
                tools::tool_executor(tool_call.function.name, tool_call.function.arguments).await;
            tx.send(ChatUpdate::ToolResult(result.clone()))
                .await
                .unwrap();

            (tool_call.id, result)
        })
    });

    let tool_results = join_all(tool_futs).await;

    for tool_result in tool_results {
        let (id, result) = tool_result.unwrap();
        messages.push(IChatCompletionMessage {
            role: "tool".to_string(),
            content: Some(result),
            tool_calls: None,
            tool_call_id: Some(id),
        });
    }
}
