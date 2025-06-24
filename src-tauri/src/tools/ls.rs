use crate::error::Error;
use crate::Result;

use super::{Function, Tool};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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

    let root_dir_path = Path::new(&root_dir);
    let relative_path = Path::new(&args.relative_path);

    let path = root_dir_path.join(relative_path);

    let canonical_root = match root_dir_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(Error::Tool(format!(
                "Failed to canonicalize the root dir: {}",
                e
            )))
        }
    };

    let canonical_path = match path.canonicalize() {
        Ok(path) => path,
        Err(e) => return Err(Error::Tool(format!("Failed to canonicalize path: {}", e))),
    };

    if !canonical_path.starts_with(&canonical_root) {
        return Err(Error::Tool("Error: path outside root dir".to_string()));
    }

    match fs::read_dir(canonical_path) {
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
