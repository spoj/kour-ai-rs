mod error;
mod settings;

use self::error::Error;
use self::settings::Settings;
use serde_json::{from_value, to_value};
use tauri::{Manager, State, Wry};
use tauri_plugin_store::{Store, StoreBuilder};

type Result<T> = std::result::Result<T, Error>;

#[tauri::command]
fn get_settings(store: State<Store<Wry>>) -> Result<Settings> {
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
fn set_settings(store: State<'_, Store<Wry>>, settings: Settings) -> Result<()> {
    store.set("settings", to_value(settings)?);
    store.save()?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_settings, set_settings])
        .setup(|app| {
            let builder = StoreBuilder::new(app, "store.bin");
            let store = builder.build();
            app.manage(store);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
