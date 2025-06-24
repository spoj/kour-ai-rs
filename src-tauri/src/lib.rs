mod chat;
mod error;
mod settings;

use self::chat::IChatCompletionOptions;
use self::error::Error;
use self::settings::Settings;
use serde_json::{from_value, to_value};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use tauri::Wry;
use tauri_plugin_store::{Store, StoreBuilder};

type Result<T> = std::result::Result<T, Error>;

static STORE: OnceLock<Arc<Store<Wry>>> = OnceLock::new();

#[tauri::command]
fn get_settings() -> Result<Settings> {
    let store = STORE.get().unwrap();
    let settings = store
        .get("settings")
        .and_then(|v| from_value(v).ok())
        .unwrap_or({
            Settings {
                api_key: "".to_string(),
                model_name: "".to_string(),
                root_dir: "".to_string(),
                system_prompt: "".to_string(),
                soffice_path: "".to_string(),
                provider_order: "".to_string(),
            }
        });
    Ok(settings)
}

#[tauri::command]
fn set_settings(settings: Settings) -> Result<()> {
    let store = STORE.get().unwrap();
    store.set("settings", to_value(settings)?);
    store.save()?;
    Ok(())
}

#[tauri::command]
async fn chat_completion(window: tauri::Window, options: IChatCompletionOptions) -> Result<()> {
    let client = reqwest::Client::new();
    let mut messages = options.messages;
    let api_key = options.api_key;
    let model_name = options.model_name;

    loop {
        let res = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&api_key)
            .json(&serde_json::json!({
                "model": model_name,
                "messages": messages,
                "tools": &*chat::TOOLS,
            }))
            .send()
            .await?
            .json::<chat::ChatCompletionResponse>()
            .await?;

        let choice = &res.choices[0];
        if let Some(tool_calls) = choice.message.tool_calls.clone() {
            messages.push(choice.message.clone());
            for tool_call in tool_calls {
                // TODO: add streaming support for tool calls
                let _ = window.emit(
                    "chat_completion_update",
                    &serde_json::json!({
                        "type": "update",
                        "isNotification": true,
                        "message": format!("Calling {}", tool_call.function.name)
                    }),
                );
                let result =
                    chat::tool_executor(tool_call.function.name, tool_call.function.arguments);
                let _ = window.emit(
                    "chat_completion_update",
                    &serde_json::json!({
                        "type": "update",
                        "isNotification": true,
                        "message": format!("Tool result: {}", result)
                    }),
                );
                messages.push(chat::IChatCompletionMessage {
                    role: "tool".to_string(),
                    content: Some(result),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id),
                });
            }
        } else {
            let _ = window.emit(
                "chat_completion_update",
                &serde_json::json!({
                    "type": "update",
                    "message": choice.message.content.clone().unwrap()
                }),
            );
            break;
        }
    }
    let _ = window.emit(
        "chat_completion_update",
        &serde_json::json!({"type": "end"}),
    );
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
