use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};

use crate::{error::Error, search::selection_clear, Result, STORE};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "modelName")]
    pub model_name: String,
    #[serde(rename = "rootDir")]
    pub root_dir: String,
    #[serde(rename = "sofficePath")]
    pub soffice_path: String,
    #[serde(rename = "providerOrder")]
    pub provider_order: String,
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<()> {
    let store = STORE
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?;
    store.set("settings", to_value(settings)?);
    store.save()?;
    selection_clear();
    Ok(())
}

#[tauri::command]
pub fn get_settings() -> Result<Settings> {
    let store = STORE
        .get()
        .ok_or(Error::Io(std::io::ErrorKind::NotFound.into()))?;
    let settings = store
        .get("settings")
        .and_then(|v| from_value(v).ok())
        .unwrap_or_default();
    Ok(settings)
}
pub fn get_root() -> Result<Utf8PathBuf> {
    let settings = get_settings()?;
    let root_dir = settings.root_dir;
    if root_dir.is_empty() {
        return Err(crate::Error::Other);
    }
    Ok(Utf8Path::new(&root_dir).to_owned())
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            model_name: "google/gemini-2.5-pro".to_string(),
            root_dir: "".to_string(),
            soffice_path: "".to_string(),
            provider_order: "google-vertex".to_string(),
        }
    }
}
