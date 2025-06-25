mod chat;
mod error;
pub mod file_handler;
mod settings;
mod tools;
mod utils;

use self::chat::{ChatCompletionMessage, ChatCompletionOptions, Content};
use self::error::Error;
use self::settings::Settings;
use serde::Serialize;
use serde_json::{from_value, to_value};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};
use tauri::{Emitter, Manager};
use tauri::{State, Wry};
use tauri_plugin_store::{Store, StoreBuilder};
use tokio::sync::mpsc;

type Result<T> = std::result::Result<T, Error>;

pub static STORE: OnceLock<Arc<Store<Wry>>> = OnceLock::new();
pub static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static SYSTEM_PROMPT: &str = include_str!("DEFAULT_PROMPT.md");

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum EventPayload<'a> {
    Start,
    End,
    Message {
        message: ChatCompletionMessage,
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
type SharedHistory = Arc<RwLock<Vec<ChatCompletionMessage>>>;
struct ChatProcessor {
    window: tauri::Window,
    options: ChatCompletionOptions,
    messages: Vec<ChatCompletionMessage>,
    tx: mpsc::Sender<chat::ChatUpdate>,
}

impl ChatProcessor {
    fn new(
        window: tauri::Window,
        options: ChatCompletionOptions,
        messages: Vec<ChatCompletionMessage>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let mut processor = Self {
            window,
            options,
            messages,
            tx,
        };
        processor.start_update_listener(rx);
        processor
    }

    fn start_update_listener(&mut self, mut rx: mpsc::Receiver<chat::ChatUpdate>) {
        let window = self.window.clone();
        tokio::spawn(async move {
            while let Some(update) = rx.recv().await {
                let _ = match update {
                    chat::ChatUpdate::ToolCall {
                        name,
                        id,
                        arguments,
                    } => window.emit(
                        "chat_completion_update",
                        EventPayload::ToolCall {
                            tool_name: &name,
                            tool_call_id: &id,
                            tool_args: &arguments,
                        },
                    ),
                    chat::ChatUpdate::ToolResult { id, result } => window.emit(
                        "chat_completion_update",
                        EventPayload::ToolDone {
                            tool_call_id: &id,
                            tool_result: &result,
                        },
                    ),
                };
            }
        });
    }

    async fn run(mut self) -> Result<Vec<ChatCompletionMessage>> {
        self.window
            .emit("chat_completion_update", &EventPayload::Start)?;

        loop {
            let res = chat::call_openrouter(
                &self.messages,
                &self.options.model_name,
                SYSTEM_PROMPT,
                &tools::TOOLS,
            )
            .await?;
            let choice = &res.choices[0];
            let message: ChatCompletionMessage = choice.message.clone().into();
            if let Some(tool_calls) = message.tool_calls.clone() {
                // To ensure the API gets a clean message, we create a new assistant
                // message that ONLY has the tool_calls, and no `content`.
                let assistant_tool_call_message =
                    ChatCompletionMessage::new("assistant", vec![]).tool_calls(tool_calls.clone());
                self.messages.push(assistant_tool_call_message);

                // Handle the tool calls, which will return the tool result messages.
                let new_messages = chat::handle_tool_calls(tool_calls, self.tx.clone()).await?;

                // Add the tool result messages to history.
                self.messages.extend(new_messages.clone());

            } else {
                // This is a standard text response, so add it to history and emit it for display.
                self.messages.push(message.clone());
                self.window.emit(
                    "chat_completion_update",
                    EventPayload::Message {
                        message: message.clone(),
                    },
                )?;
                break;
            }
        }

        self.window
            .emit("chat_completion_update", &EventPayload::End)?;

        Ok(self.messages)
    }
}

pub fn get_settings_fn() -> Result<Settings> {
    let store = STORE
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?;
    let settings = store
        .get("settings")
        .and_then(|v| from_value(v).ok())
        .unwrap_or_default();
    Ok(settings)
}

pub fn get_cache_dir() -> Result<std::path::PathBuf> {
    let cache_dir = crate::CACHE_DIR
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?
        .join("conversion_cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

#[tauri::command]
fn get_settings() -> Result<Settings> {
    get_settings_fn()
}

#[tauri::command]
fn set_settings(settings: Settings) -> Result<()> {
    let store = STORE
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?;
    store.set("settings", to_value(settings)?);
    store.save()?;
    Ok(())
}

#[tauri::command]
async fn chat_completion(
    window: tauri::Window,
    message: ChatCompletionMessage,
    state: State<'_, SharedHistory>,
) -> Result<()> {
    let settings = get_settings_fn()?;
    let options = ChatCompletionOptions {
        model_name: settings.model_name,
    };
    let mut history = state.read().unwrap().to_vec();
    history.push(message);
    let new_history = ChatProcessor::new(window, options, history).run().await?;
    let mut history = state.write().unwrap();
    *history = new_history;
    Ok(())
}
#[tauri::command]
async fn replay_history(window: tauri::Window, state: State<'_, SharedHistory>) -> Result<()> {
    let history = state.read().unwrap().clone();
    let mut last_tool_call_id = None;

    for message in history {
        // Assistant message with tool calls
        if let Some(tool_calls) = &message.tool_calls {
            for tool_call in tool_calls {
                window.emit(
                    "chat_completion_update",
                    EventPayload::ToolCall {
                        tool_name: &tool_call.function.name,
                        tool_call_id: &tool_call.id,
                        tool_args: &tool_call.function.arguments,
                    },
                )?;
                last_tool_call_id = Some(tool_call.id.clone());
            }
        }
        // Tool message with the result
        else if message.role == "tool" {
            if let Some(tool_call_id) = &message.tool_call_id {
                if let Some(Content::Text { text }) = message.content.first() {
                    window.emit(
                        "chat_completion_update",
                        EventPayload::ToolDone {
                            tool_call_id: &tool_call_id,
                            tool_result: &text,
                        },
                    )?;
                }
            }
        }
        // All other messages
        else {
            window.emit("chat_completion_update", EventPayload::Message { message })?;
        }
    }

    Ok(())
}

#[tauri::command]
fn clear_history(state: State<'_, SharedHistory>) -> Result<()> {
    let mut history = state.write().unwrap();
    history.clear();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            chat_completion,
            replay_history,
            clear_history
        ])
        .setup(|app| {
            STORE.get_or_init(|| {
                StoreBuilder::new(app.handle(), "store.bin")
                    .build()
                    .unwrap()
            });
            CACHE_DIR.get_or_init(|| app.path().app_cache_dir().unwrap());
            let history: SharedHistory = Arc::new(RwLock::new(vec![]));
            app.manage(history);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
