use crate::chat::{ChatCompletionMessage, Content};
use crate::error::Error;
use crate::file_handler;
use crate::tools::{Function, Tool};
use crate::utils::jailed::Jailed;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::task;

#[derive(Deserialize)]
pub struct LoadFileArgs {
    pub filename: String,
}

#[derive(Serialize)]
pub struct LoadFileResult {
    pub r#type: String, // Always "file_loaded" to identify this special result.
    pub display_message: String,
    pub user_message: ChatCompletionMessage,
}

pub async fn load_file(args: LoadFileArgs) -> Result<String> {
    let root_dir = task::spawn_blocking(crate::get_settings_fn)
        .await?
        .map(|s| s.root_dir)?;

    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }

    let jail = Path::new(&root_dir);
    let safe_path = jail.jailed_join(Path::new(&args.filename))?;

    let file_content = task::spawn_blocking(move || file_handler::process_file_for_llm(&safe_path))
        .await??;

    // Create the user message that contains the file attachment.
    let user_message = ChatCompletionMessage::new("user", file_content);

    let result = LoadFileResult {
        r#type: "file_loaded".to_string(),
        display_message: format!("Loaded {}", &args.filename),
        user_message,
    };

    serde_json::to_string(&result).map_err(|e| e.into())
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "load_file".to_string(),
            description: "Loads a file directly into the conversation context. Supports various file types.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The path to the file to load."
                    }
                },
                "required": ["filename"]
            }),
        },
    }
}