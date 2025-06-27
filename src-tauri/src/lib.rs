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
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use tauri::{Emitter, Manager};
use tauri::{State, Wry};
use tauri_plugin_store::{Store, StoreBuilder};
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

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
struct AppStateInner {
    cancel: Mutex<Option<CancellationToken>>,
    history: RwLock<Vec<ChatCompletionMessage>>,
}
type AppState<'a> = State<'a, AppStateInner>;

struct EventReplayer {
    window: tauri::Window,
}

impl EventReplayer {
    fn new(window: tauri::Window) -> Self {
        Self { window }
    }

    fn emit_tool_call(&self, name: &str, id: &str, arguments: &str) -> Result<()> {
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

    fn emit_tool_result(&self, id: &str, result: &str) -> Result<()> {
        self.window.emit(
            "chat_completion_update",
            EventPayload::ToolDone {
                tool_call_id: id,
                tool_result: result,
            },
        )?;
        Ok(())
    }

    fn replay(&self, messages: &[ChatCompletionMessage]) -> Result<()> {
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
                self.window.emit(
                    "chat_completion_update",
                    EventPayload::Message {
                        message: message.clone(),
                    },
                )?;
            }
        }
        Ok(())
    }
}
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
        let replayer = EventReplayer::new(self.window.clone());
        tokio::spawn(async move {
            while let Some(update) = rx.recv().await {
                let _ = match update {
                    chat::ChatUpdate::ToolCall {
                        name,
                        id,
                        arguments,
                    } => replayer.emit_tool_call(&name, &id, &arguments),
                    chat::ChatUpdate::ToolResult { id, result } => {
                        replayer.emit_tool_result(&id, &result)
                    }
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
                let assistant_tool_call_message =
                    ChatCompletionMessage::new("assistant", vec![]).tool_calls(tool_calls.clone());
                self.messages.push(assistant_tool_call_message);

                let new_messages = chat::handle_tool_calls(tool_calls, self.tx.clone()).await?;
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
                EventReplayer::new(self.window.clone()).replay(&[message])?;
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
    state: AppState<'_>,
) -> Result<()> {
    let settings = get_settings_fn()?;
    let options = ChatCompletionOptions {
        model_name: settings.model_name,
    };
    let mut history = state.history.read().unwrap().to_vec();
    history.push(message);
    let cancel_token = CancellationToken::new();
    {
        let mut guard = state.cancel.lock().unwrap();
        if guard.is_none() {
            *guard = Some(cancel_token.clone());
        } else {
            return Err(Error::Send("more than 1 request".to_string()));
        }
    }

    select! {
        _ = cancel_token.cancelled() => {
            window.clone().emit("chat_completion_update", &EventPayload::End)?;
            *state.cancel.lock().unwrap() = None
        }
        Ok(new_history) = ChatProcessor::new(window.clone(), options, history).run() => {
            *state.cancel.lock().unwrap() = None;
            let mut history = state.history.write().unwrap();
            *history = new_history;
        }
    }
    Ok(())
}
#[tauri::command]
async fn replay_history(window: tauri::Window, state: AppState<'_>) -> Result<()> {
    let history = state.history.read().unwrap().clone();
    EventReplayer::new(window).replay(&history)?;
    Ok(())
}

#[tauri::command]
fn clear_history(state: AppState<'_>) -> Result<()> {
    cancel_outstanding_request(state.clone())?;
    let mut history = state.history.write().unwrap();
    history.clear();
    Ok(())
}

#[tauri::command]
fn cancel_outstanding_request(state: AppState<'_>) -> Result<()> {
    if let Some(cancel_token) = state.cancel.lock().unwrap().as_ref() {
        cancel_token.cancel();
    }
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
            clear_history,
            cancel_outstanding_request
        ])
        .setup(|app| {
            STORE.get_or_init(|| {
                StoreBuilder::new(app.handle(), "store.bin")
                    .build()
                    .unwrap()
            });
            CACHE_DIR.get_or_init(|| app.path().app_cache_dir().unwrap());
            let history = RwLock::new(vec![]);
            let cancel = Mutex::new(None);
            let inner_state = AppStateInner { cancel, history };
            app.manage(inner_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
