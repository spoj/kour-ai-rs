mod chat;
mod error;
mod file_handler;
mod search;
mod interaction;
mod openrouter;
mod settings;
mod tools;
mod ui_events;
mod utils;

use crate::chat::ChatProcessor;
use crate::error::Error;
use crate::search::Searching;
use crate::interaction::{Content, History, Source};
use crate::openrouter::ChatOptions;
use crate::settings::get_settings;
use crate::ui_events::UIEvents;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
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
    history: Arc<Mutex<History>>,
}
type AppState<'a> = State<'a, AppStateInner>;

pub fn get_cache_dir() -> Result<std::path::PathBuf> {
    let cache_dir = crate::CACHE_DIR
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?
        .join("conversion_cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

#[tauri::command]
async fn chat(window: tauri::Window, content: Vec<Content>, state: AppState<'_>) -> Result<()> {
    let settings = get_settings()?;
    let options = ChatOptions {
        model_name: settings.model_name,
    };
    let cancel_token = CancellationToken::new();
    {
        let mut guard = state.cancel.lock().unwrap(); // unwrap: won't try to recover from poisoned lock
        if guard.is_none() {
            *guard = Some(cancel_token.clone());
        } else {
            return Err(Error::Conflict("more than 1 request".to_string()));
        }
    }

    let replayer = UIEvents::new(window.clone());
    if !content.is_empty() {
        let new_interaction = UIEvents::sends(content);
        let _ = replayer.emit_interaction(&new_interaction);
        state.history.lock().unwrap().push(new_interaction); // unwrap: won't try to recover from poisoned lock
    }
    let proc = ChatProcessor::new(window.clone(), options, Arc::clone(&state.history));
    select! {
        Ok(_) = {proc.run()} => {}
        _ = cancel_token.cancelled() => {
            let _ = replayer.emit_done();
            state.history.lock().unwrap().clean_unfinished_tool_calls();
        }
    }
    *state.cancel.lock().unwrap() = None; // unwrap: won't try to recover from poisoned lock
    Ok(())
}
#[tauri::command]
async fn replay_history(window: tauri::Window, state: AppState<'_>) -> Result<()> {
    let history = state.history.lock().unwrap().clone(); // unwrap: won't try to recover from poisoned lock
    UIEvents::new(window).replay_history(&history)?;
    Ok(())
}

#[tauri::command]
fn clear_history(state: AppState<'_>) -> Result<()> {
    cancel_outstanding_request(state.clone())?;
    let mut history = state.history.lock().unwrap(); // unwrap: won't try to recover from poisoned lock
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

#[tauri::command]
fn delete_message(id: usize, state: AppState<'_>) -> Result<()> {
    cancel_outstanding_request(state.clone())?;
    state.history.lock().unwrap().delete_by_id(id);
    Ok(())
}

#[tauri::command]
fn delete_tool_interaction(
    llm_interaction_id: usize,
    tool_call_id: String,
    state: AppState<'_>,
) -> Result<()> {
    cancel_outstanding_request(state.clone())?;
    state
        .history
        .lock()
        .unwrap()
        .delete_by_tool_id(llm_interaction_id, &tool_call_id);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::set_settings,
            chat,
            replay_history,
            clear_history,
            cancel_outstanding_request,
            delete_message,
            delete_tool_interaction,
            search::search_files_by_name
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
            let history = Arc::new(Mutex::new(Default::default()));
            let cancel = Mutex::new(None);
            let inner_state = AppStateInner { cancel, history };
            app.manage(inner_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
