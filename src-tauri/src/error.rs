use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Store(#[from] tauri_plugin_store::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Glob(#[from] glob::PatternError),
    #[error(transparent)]
    GlobSet(#[from] globset::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("Failed to send chat update: {0}")]
    Tauri(String),
    #[error("Tool Error: {0}")]
    Tool(String),
    #[error("State Conflict: {0}")]
    Conflict(String),
    #[error("Anyhow Error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Unkonwn Error")]
    Other,
    #[error("Limit exceeded. Requested {requested} of {item}. Limited to {limit}")]
    Limit {
        item: String,
        requested: usize,
        limit: usize,
    },
}

impl From<tauri::Error> for Error {
    fn from(err: tauri::Error) -> Self {
        Error::Tauri(err.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
