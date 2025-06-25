use crate::Result;
use crate::{error::Error};

use super::{Function, Tool};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::task;

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "find".to_string(),
            description: "Find files and directories matching a glob pattern. The search is recursive unless the pattern contains a path separator.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "glob": {
                        "type": "string",
                        "description": "The glob pattern to match against"
                    }
                },
                "required": ["glob"]
            }),
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FindArgs {
    pub glob: String,
}

pub async fn find(args: FindArgs) -> Result<String> {
    let root_dir = task::spawn_blocking(crate::get_settings_fn)
        .await?
        .map(|s| s.root_dir)?;
    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }
    let full_pattern = if Path::new(&args.glob).components().count() > 1 {
        format!("{}/{}", root_dir, &args.glob)
    } else {
        format!("{}/**/{}", root_dir, &args.glob)
    };

    let mut result = String::new();
    for entry in glob::glob(&full_pattern)
        .map_err(|e| Error::Tool(format!("Invalid glob pattern: {}", e)))?
        .flatten()
    {
        if let Some(path_str) = entry.to_str() {
            result.push_str(path_str.strip_prefix(&root_dir).unwrap_or(path_str));
            result.push('\n');
        }
    }
    Ok(result)
}
