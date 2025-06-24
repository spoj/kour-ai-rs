mod chat;
mod error;
mod settings;

use self::chat::IChatCompletionOptions;
use self::error::Error;
use self::settings::Settings;
use serde_json::{from_value, to_value};
use std::sync::{Arc, OnceLock};
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
fn chat_completion(_options: IChatCompletionOptions) -> Result<String> {
    Ok("this is a dummy response".to_string())
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
