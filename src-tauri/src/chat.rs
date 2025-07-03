use crate::{
    Result, get_settings_fn,
    interaction::{Interaction, Interactor, Llm},
    tools,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, to_string_pretty};
use tauri::Emitter;

pub static SYSTEM_PROMPT: &str = include_str!("DEFAULT_PROMPT.md");

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum EventPayload<'a> {
    Start,
    End,
    Message {
        message: OutgoingMessage,
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
pub struct OutgoingMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
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

    pub fn emit_interaction(&self, interaction: &Interaction) -> Result<()> {
        match interaction {
            Interaction::LlmResponse {
                content,
                tool_calls,
            } => {
                let _ = self.emit_message(OutgoingMessage {
                    role: "assistant".to_string(),
                    content: content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                });
                if let Some(tool_calls) = tool_calls {
                    for tool_call in tool_calls {
                        self.emit_tool_call(
                            &tool_call.function.name,
                            &tool_call.id,
                            &tool_call.function.arguments,
                        )?;
                    }
                }
                Ok(())
            }
            Interaction::ToolResult {
                tool_call_id,
                response,
                #[allow(unused_variables)]
                for_llm,
                #[allow(unused_variables)]
                for_user,
            } => {
                let _ = self.emit_tool_result(tool_call_id, response);
                Ok(())
            }
            Interaction::UserMessage { content } => {
                let _ = self.emit_message(OutgoingMessage {
                    role: "user".to_string(),
                    content: content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                });
                Ok(())
            }
        }
    }

    pub fn emit_message(&self, message: OutgoingMessage) -> Result<()> {
        self.window
            .emit("chat_completion_update", EventPayload::Message { message })?;
        Ok(())
    }

    pub fn replay_interactions(&self, interactions: &[Interaction]) -> Result<()> {
        for i in interactions {
            let _ = self.emit_interaction(i);
        }
        Ok(())
    }

}
pub struct ChatProcessor {
    replayer: EventReplayer,
    options: ChatOptions,
    interactions: Vec<Interaction>,
}

impl ChatProcessor {
    pub fn new(window: tauri::Window, options: ChatOptions, messages: Vec<Interaction>) -> Self {
        Self {
            replayer: EventReplayer::new(window),
            options,
            interactions: messages,
        }
    }

    pub async fn run(mut self) -> Result<Vec<Interaction>> {
        let _ = self.replayer.emit_start();

        loop {
            let to_llm: Vec<_> = Llm::render(&self.interactions);
            let res = call_openrouter(
                &to_llm,
                &self.options.model_name,
                SYSTEM_PROMPT,
                &tools::TOOLS,
                None,
            )
            .await?;

            let choice = &res.choices[0];
            let incoming_message = choice.message.clone();
            let interaction = Llm::sends(incoming_message.clone());

            if let Some(tool_calls) = incoming_message.tool_calls.clone() {
                self.interactions.push(interaction);

                let new_interactions = self.handle_tool_calls(tool_calls).await?;
                for msg in new_interactions {
                    let tool_message = msg;
                    self.interactions.push(tool_message);
                }
                // After handling tools, continue the loop to let the assistant respond.
            } else {
                // It's a final text response. Add it to history, emit, and break the loop.
                self.interactions.push(interaction.clone());
                self.replayer.emit_interaction(&interaction)?;
                break;
            }
        }

        self.replayer.emit_done()?;

        Ok(self.interactions)
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

    pub async fn handle_tool_calls(&self, tool_calls: Vec<ToolCall>) -> Result<Vec<Interaction>> {
        let mut new_messages = Vec::new();

        let tool_futs = tool_calls.into_iter().map(|tool_call| {
            tokio::spawn(Self::execute_tool_call(self.replayer.clone(), tool_call))
        });

        let tool_results = join_all(tool_futs).await;

        for tool_result in tool_results {
            let (id, result_str) = tool_result??;

            new_messages.push(Interaction::ToolResult {
                tool_call_id: id,
                response: result_str,
                for_llm: vec![],
                for_user: vec![],
            });
        }

        Ok(new_messages)
    }
}

pub async fn call_openrouter(
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
            .map(|j| to_string_pretty(j).unwrap())
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
