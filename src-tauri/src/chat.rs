use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub static TOOLS: LazyLock<[Tool; 1]> = LazyLock::new(|| {
    [Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "read_file".to_string(),
            description: "read the contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file"
                    }
                },
                "required": ["path"]
            }),
        },
    }]
});

pub fn tool_executor(name: String, arguments: String) -> String {
    match name.as_str() {
        "read_file" => {
            let args: serde_json::Value = serde_json::from_str(&arguments).unwrap();
            let path = args["path"].as_str().unwrap();
            std::fs::read_to_string(path).unwrap()
        }
        _ => "Tool not found".to_string(),
    }
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
            "tools": &*TOOLS,
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
    message: &mut Vec<IChatCompletionMessage>,
    tool_calls: Vec<ToolCall>,
    tx: tokio::sync::mpsc::Sender<ChatUpdate>,
) {
    message.push(IChatCompletionMessage {
        role: "assistant".to_string(),
        content: None,
        tool_calls: Some(tool_calls.clone()),
        tool_call_id: None,
    });

    for tool_call in tool_calls {
        tx.send(ChatUpdate::ToolCall(tool_call.function.name.clone()))
            .await
            .unwrap();
        let result = tool_executor(tool_call.function.name, tool_call.function.arguments);
        tx.send(ChatUpdate::ToolResult(result.clone()))
            .await
            .unwrap();
        message.push(IChatCompletionMessage {
            role: "tool".to_string(),
            content: Some(result),
            tool_calls: None,
            tool_call_id: Some(tool_call.id),
        });
    }
}
