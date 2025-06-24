use serde_json::from_value;
use tauri::{AppHandle, Result};
use tauri_plugin_store::StoreBuilder;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn increment_and_show(app: &AppHandle) -> Result<usize> {
    let builder = StoreBuilder::new(app, "store.bin");
    let store = builder.build().expect("store not found");
    let counter: usize = store
        .get("counter")
        .and_then(|v| from_value(v).ok())
        .unwrap();
    let counter = counter + 1;
    store.set("counter", counter);
    Ok(counter)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            let builder = StoreBuilder::new(app, "store.bin");
            let _store = builder.build();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
