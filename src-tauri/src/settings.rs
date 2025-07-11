use serde::{Deserialize, Serialize};
use serde_json::from_value;

use crate::{Result, STORE, error::Error};

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
