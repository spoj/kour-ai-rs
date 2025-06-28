use crate::{get_settings_fn, tools, Result};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

pub static SYSTEM_PROMPT: &str = include_str!("DEFAULT_PROMPT.md");

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum EventPayload<'a> {
    Start,
    End,
    Message {
        message: ChatMessage,
    },
    ToolCall {
        tool_name: &'a str,
        tool_call_id: &'a str,
        tool_args: &'a str,
    },
    ToolDone {
        tool_call_id: &'a str,
        tool_result: &'a str,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injected_user_message: Option<Box<ChatMessage>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
    #[serde(rename = "file")]
    File { file: FileData },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct FileData {
    pub filename: String,
    pub file_data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingMessage {
    pub role: String,
    #[serde(default)]
    content: IncomingContent,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
#[derive(Default)]
enum IncomingContent {
    Text(String),
    Parts(Vec<Content>),
    #[default]
    None,
}

impl From<IncomingMessage> for ChatMessage {
    fn from(msg: IncomingMessage) -> Self {
        let content = match msg.content {
            IncomingContent::Text(text) => vec![Content::Text { text }],
            IncomingContent::Parts(parts) => parts,
            IncomingContent::None => vec![],
        };

        ChatMessage {
            role: msg.role,
            content,
            tool_calls: msg.tool_calls,
            tool_call_id: None,
            injected_user_message: None,
        }
    }
}

impl ChatMessage {
    pub fn new(role: &str, content: Vec<Content>) -> Self {
        Self {
            role: role.to_string(),
            content,
            tool_calls: None,
            tool_call_id: None,
            injected_user_message: None,
        }
    }

    pub fn from_user_content(content: Vec<Content>) -> Self {
        Self {
            role: "user".to_string(),
            content,
            tool_calls: None,
            tool_call_id: None,
            injected_user_message: None,
        }
    }

    pub fn tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self.content = vec![]; // Empty vec will be omitted by serde
        self
    }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: IncomingMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChoiceDelta {
    pub delta: IncomingMessage,
}

#[derive(Clone)]
pub struct EventReplayer {
    window: tauri::Window,
}

impl EventReplayer {
    pub fn new(window: tauri::Window) -> Self {
        Self { window }
    }

    pub fn emit_tool_call(&self, name: &str, id: &str, arguments: &str) -> Result<()> {
        self.window.emit(
            "chat_completion_update",
            EventPayload::ToolCall {
                tool_name: name,
                tool_call_id: id,
                tool_args: arguments,
            },
        )?;
        Ok(())
    }

    pub fn emit_tool_result(&self, id: &str, result: &str) -> Result<()> {
        self.window.emit(
            "chat_completion_update",
            EventPayload::ToolDone {
                tool_call_id: id,
                tool_result: result,
            },
        )?;
        Ok(())
    }
    pub fn emit_done(&self) -> Result<()> {
        self.window
            .emit("chat_completion_update", EventPayload::End)?;
        Ok(())
    }

    pub fn emit_start(&self) -> Result<()> {
        self.window
            .emit("chat_completion_update", EventPayload::Start)?;
        Ok(())
    }

    pub fn emit_message(&self, message: ChatMessage) -> Result<()> {
        self.window
            .emit("chat_completion_update", EventPayload::Message { message })?;
        Ok(())
    }

    pub fn replay(&self, messages: &[ChatMessage]) -> Result<()> {
        for message in messages {
            // Assistant message with tool calls
            if let Some(tool_calls) = &message.tool_calls {
                for tool_call in tool_calls {
                    self.emit_tool_call(
                        &tool_call.function.name,
                        &tool_call.id,
                        &tool_call.function.arguments,
                    )?;
                }
            }
            // Tool message with the result
            else if message.role == "tool" {
                if let Some(tool_call_id) = &message.tool_call_id {
                    if let Some(Content::Text { text }) = message.content.first() {
                        self.emit_tool_result(tool_call_id, text)?;
                    }
                }
                // We do NOT replay the injected user message, it's for the LLM only
            }
            // All other messages
            else {
                self.emit_message(message.clone())?;
            }
        }
        Ok(())
    }
}
pub struct ChatProcessor {
    replayer: EventReplayer,
    options: ChatOptions,
    messages: Vec<ChatMessage>,
}

impl ChatProcessor {
    pub fn new(window: tauri::Window, options: ChatOptions, messages: Vec<ChatMessage>) -> Self {
        Self {
            replayer: EventReplayer::new(window),
            options,
            messages,
        }
    }

    pub async fn run(mut self) -> Result<Vec<ChatMessage>> {
        let _ = self.replayer.emit_start();

        loop {
            let res = call_openrouter(
                &self.messages,
                &self.options.model_name,
                SYSTEM_PROMPT,
                &tools::TOOLS,
            )
            .await?;

            let choice = &res.choices[0];
            let message: ChatMessage = choice.message.clone().into();

            if let Some(tool_calls) = message.tool_calls.clone() {
                let assistant_tool_call_message =
                    ChatMessage::new("assistant", vec![]).tool_calls(tool_calls.clone());
                self.messages.push(assistant_tool_call_message);

                let new_messages = self.handle_tool_calls(tool_calls).await?;
                for msg in new_messages {
                    let mut tool_message = msg;
                    if let Some(user_msg) = tool_message.injected_user_message.take() {
                        self.messages.push(tool_message);
                        self.messages.push(*user_msg);
                    } else {
                        self.messages.push(tool_message);
                    }
                }
                // After handling tools, continue the loop to let the assistant respond.
            } else {
                // It's a final text response. Add it to history, emit, and break the loop.
                self.messages.push(message.clone());
                self.replayer.replay(&[message])?;
                break;
            }
        }

        self.replayer.emit_done()?;

        Ok(self.messages)
    }

    async fn execute_tool_call(
        replayer: EventReplayer,
        tool_call: ToolCall,
    ) -> Result<(String, String)> {
        replayer
            .emit_tool_call(
                &tool_call.function.name,
                &tool_call.id,
                &tool_call.function.arguments,
            )
            .expect("error emit");
        let json_value =
            match tools::tool_executor(&tool_call.function.name, &tool_call.function.arguments)
                .await
            {
                Ok(value) => value,
                Err(e) => serde_json::Value::String(e.to_string()),
            };

        let result = serde_json::to_string(&json_value).unwrap_or_else(|_| json_value.to_string());
        let _ = replayer.emit_tool_result(&tool_call.id, &result);

        Ok((tool_call.id, result))
    }

    pub async fn handle_tool_calls(&self, tool_calls: Vec<ToolCall>) -> Result<Vec<ChatMessage>> {
        let mut new_messages = Vec::new();

        let tool_futs = tool_calls.into_iter().map(|tool_call| {
            tokio::spawn(Self::execute_tool_call(self.replayer.clone(), tool_call))
        });

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
                    if let Ok(user_message) =
                        serde_json::from_value::<ChatMessage>(file_result["user_message"].clone())
                    {
                        // 1. Add the simple tool message for display, with the rich user message nested inside.
                        new_messages.push(ChatMessage {
                            role: "tool".to_string(),
                            content: vec![Content::Text {
                                text: display_message,
                            }],
                            tool_call_id: Some(id),
                            tool_calls: None,
                            injected_user_message: Some(Box::new(user_message)),
                        });

                        continue; // Skip the default handling below
                    }
                }
            }

            // Default handling for all other tools
            new_messages.push(ChatMessage {
                role: "tool".to_string(),
                content: vec![Content::Text { text: result_str }],
                tool_call_id: Some(id),
                tool_calls: None,
                injected_user_message: None,
            });
        }

        Ok(new_messages)
    }
}

pub async fn call_openrouter(
    messages: &[ChatMessage],
    model_name: &str,
    system_prompt: &str,
    tools: &Vec<tools::Tool>,
) -> super::Result<ChatResponse> {
    println!("Sending messages to OpenRouter: {messages:?}");
    let settings = get_settings_fn()?;
    let client = reqwest::Client::new();
    let mut final_messages = messages.to_vec();
    if !system_prompt.is_empty() {
        final_messages.insert(
            0,
            ChatMessage::new(
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
