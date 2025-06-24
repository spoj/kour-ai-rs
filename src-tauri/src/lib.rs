mod chat;
mod error;
mod settings;
mod tools;

use self::chat::{ChatCompletionMessage, ChatCompletionOptions};
use self::error::Error;
use self::settings::Settings;
use serde::Serialize;
use serde_json::{from_value, to_value};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use tauri::Wry;
use tauri_plugin_store::{Store, StoreBuilder};
use tokio::sync::mpsc;

type Result<T> = std::result::Result<T, Error>;

static STORE: OnceLock<Arc<Store<Wry>>> = OnceLock::new();

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum EventPayload<'a> {
    Start,
    End,
    Update {
        message: &'a str,
        is_notification: bool,
    },
}

impl<'a> EventPayload<'a> {
    fn new_update(message: &'a str, is_notification: bool) -> Self {
        EventPayload::Update {
            message,
            is_notification,
        }
    }
}

struct ChatProcessor {
    window: tauri::Window,
    options: ChatCompletionOptions,
    messages: Vec<ChatCompletionMessage>,
    tx: mpsc::Sender<chat::ChatUpdate>,
}

impl ChatProcessor {
    fn new(window: tauri::Window, options: ChatCompletionOptions) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let mut processor = Self {
            window,
            options,
            messages: Vec::new(),
            tx,
        };
        processor.start_update_listener(rx);
        processor
    }

    fn start_update_listener(&mut self, mut rx: mpsc::Receiver<chat::ChatUpdate>) {
        let window = self.window.clone();
        tokio::spawn(async move {
            while let Some(update) = rx.recv().await {
                let (msg, is_notification) = match update {
                    chat::ChatUpdate::ToolCall(name) => (format!("Calling {}.", name), true),
                    chat::ChatUpdate::ToolResult(name, _) => (format!("{} Done.", name), true),
                };
                let _ = window.emit(
                    "chat_completion_update",
                    EventPayload::new_update(&msg, is_notification),
                );
            }
        });
    }

    async fn run(&mut self) -> Result<()> {
        self.window
            .emit("chat_completion_update", &EventPayload::Start)?;
        self.messages = self.options.messages.clone();

        loop {
            let res = chat::call_openrouter(
                &self.messages,
                &self.options.api_key,
                &self.options.model_name,
            )
            .await?;
            let choice = &res.choices[0];

            if let Some(tool_calls) = choice.message.tool_calls.clone() {
                let new_messages = chat::handle_tool_calls(tool_calls, self.tx.clone()).await?;
                self.messages.extend(new_messages);
            } else {
                let content = choice.message.content.clone().unwrap_or_default();
                self.window.emit(
                    "chat_completion_update",
                    EventPayload::new_update(&content, false),
                )?;
                break;
            }
        }

        self.window
            .emit("chat_completion_update", &EventPayload::End)?;

        Ok(())
    }
}

#[tauri::command]
fn get_settings() -> Result<Settings> {
    let store = STORE
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?;
    let settings = store
        .get("settings")
        .and_then(|v| from_value(v).ok())
        .unwrap_or_default();
    Ok(settings)
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
async fn chat_completion(window: tauri::Window, options: ChatCompletionOptions) -> Result<()> {
    ChatProcessor::new(window, options).run().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            chat_completion
        ])
        .setup(|app| {
            STORE.get_or_init(|| {
                StoreBuilder::new(app.handle(), "store.bin")
                    .build()
                    .unwrap()
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
