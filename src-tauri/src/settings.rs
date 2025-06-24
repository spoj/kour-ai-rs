use serde::{Deserialize, Serialize};

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

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            model_name: "openai/gpt-4o".to_string(),
            root_dir: "".to_string(),
            soffice_path: "".to_string(),
            provider_order: "openai,google,anthropic".to_string(),
        }
    }
}
