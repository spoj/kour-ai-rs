use crate::{error::Error, utils};
use crate::Result;

use super::{Function, Tool};
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::task;

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "ls".to_string(),
            description: "List the content of a path relative to the root directory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "relative_path": {
                        "type": "string",
                        "description": "The path relative to the root directory"
                    }
                },
                "required": ["relative_path"]
            }),
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LsArgs {
    pub relative_path: String,
}

pub async fn ls(args: LsArgs) -> Result<String> {
    let root_dir = task::spawn_blocking(crate::get_settings_fn)
        .await?
        .map(|s| s.root_dir)?;

    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }

    let safe_path = utils::get_safe_path(&root_dir, &args.relative_path)?;

    match fs::read_dir(safe_path) {
        Ok(entries) => {
            let mut result = String::new();
            for entry in entries.flatten() {
                result.push_str(&entry.file_name().to_string_lossy());
                result.push('\n');
            }
            Ok(result)
        }
        Err(e) => Err(Error::Tool(format!("Error: failed to read dir: {}", e))),
    }
}
