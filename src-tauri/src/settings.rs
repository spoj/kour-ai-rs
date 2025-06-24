use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "modelName")]
    pub model_name: String,
    #[serde(rename = "rootDir")]
    pub root_dir: String,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "sofficePath")]
    pub soffice_path: String,
    #[serde(rename = "providerOrder")]
    pub provider_order: String,
}