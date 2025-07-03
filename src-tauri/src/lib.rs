mod chat;
mod error;
mod file_handler;
mod interaction;
mod settings;
mod tools;
mod utils;

use crate::chat::{ChatOptions, Content, OutgoingMessage};
use crate::chat::{ChatProcessor, EventReplayer};
use crate::error::Error;
use crate::interaction::Interaction;
use crate::settings::Settings;
use serde_json::{from_value, to_value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use tauri::Manager;
use tauri::{State, Wry};
use tauri_plugin_store::{Store, StoreBuilder};
use tokio::select;
use tokio_util::sync::CancellationToken;

type Result<T> = std::result::Result<T, Error>;

pub static STORE: OnceLock<Arc<Store<Wry>>> = OnceLock::new();
pub static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

struct AppStateInner {
    cancel: Mutex<Option<CancellationToken>>,
    history: RwLock<Vec<Interaction>>,
}
type AppState<'a> = State<'a, AppStateInner>;

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
async fn chat(window: tauri::Window, content: Vec<Content>, state: AppState<'_>) -> Result<()> {
    let settings = get_settings_fn()?;
    let options = ChatOptions {
        model_name: settings.model_name,
    };
    let replayer = EventReplayer::new(window.clone());
    let mut history = state.history.read().unwrap().to_vec(); // unwrap: won't try to recover from poisoned lock

    let new_interaction = Interaction::from_user(content);
    history.push(new_interaction.clone());
    let _ = replayer.emit_interaction(&new_interaction);

    let cancel_token = CancellationToken::new();
    {
        let mut guard = state.cancel.lock().unwrap(); // unwrap: won't try to recover from poisoned lock
        if guard.is_none() {
            *guard = Some(cancel_token.clone());
        } else {
            return Err(Error::Conflict("more than 1 request".to_string()));
        }
    }

    select! {
        _ = cancel_token.cancelled() => {
            let _ = replayer.emit_done();
            *state.cancel.lock().unwrap() = None; // unwrap: won't try to recover from poisoned lock
        }
        Ok(new_history) = ChatProcessor::new(window.clone(), options, history).run() => {
            *state.cancel.lock().unwrap() = None; // unwrap: won't try to recover from poisoned lock
            let mut history = state.history.write().unwrap(); // unwrap: won't try to recover from poisoned lock
            *history = new_history;
        }
    }
    Ok(())
}
#[tauri::command]
async fn replay_history(window: tauri::Window, state: AppState<'_>) -> Result<()> {
    let history = state.history.read().unwrap().clone(); // unwrap: won't try to recover from poisoned lock
    EventReplayer::new(window).replay_interactions(&history)?;
    Ok(())
}

#[tauri::command]
fn clear_history(state: AppState<'_>) -> Result<()> {
    cancel_outstanding_request(state.clone())?;
    let mut history = state.history.write().unwrap(); // unwrap: won't try to recover from poisoned lock
    history.clear();
    Ok(())
}

#[tauri::command]
fn cancel_outstanding_request(state: AppState<'_>) -> Result<()> {
    if let Some(cancel_token) = state.cancel.lock().unwrap().as_ref() {
        // unwrap: won't try to recover from poisoned lock
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
            chat,
            replay_history,
            clear_history,
            cancel_outstanding_request
        ])
        .setup(|app| {
            STORE.get_or_init(|| {
                StoreBuilder::new(app.handle(), "store.bin")
                    .build()
                    .unwrap() // unwrap: crash if cannot initialize store
            });
            CACHE_DIR.get_or_init(
                || app.path().app_cache_dir().unwrap(), // unwrap: crash if cannot find cache dir
            );
            let history = RwLock::new(vec![]);
            let cancel = Mutex::new(None);
            let inner_state = AppStateInner { cancel, history };
            app.manage(inner_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
