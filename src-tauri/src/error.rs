use crate::chat;
use serde::{Serialize, Serializer};
use tokio::sync::mpsc;

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
    Join(#[from] tokio::task::JoinError),
    #[error("Failed to send chat update: {0}")]
    Send(String),
    #[error("Tauri Error: {0}")]
    Tauri(String),
    #[error("Tool Error: {0}")]
    Tool(String),
    #[error("Anyhow Error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Walkdir Error: {0}")]
    Walkdir(#[from] walkdir::Error),
}

impl From<tauri::Error> for Error {
    fn from(err: tauri::Error) -> Self {
        Error::Tauri(err.to_string())
    }
}

impl From<mpsc::error::SendError<chat::ChatUpdate>> for Error {
    fn from(err: mpsc::error::SendError<chat::ChatUpdate>) -> Self {
        Error::Send(err.to_string())
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
