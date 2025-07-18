use crate::{settings::get_root, Result};
use crate::error::Error;
use crate::utils::jailed::Jailed;
use std::path::Path;

use super::{Function, Tool};
use serde::{Deserialize, Serialize};
use std::fs;

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "ls".to_string(),
            description: "Lists contents of a specific directory level without sampling, good for iterative exploration".to_string(),
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

pub async fn ls(args: LsArgs) -> Result<Vec<String>> {
    let root_dir = get_root()?;

    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }

    let jail = Path::new(&root_dir);
    let safe_path = jail.jailed_join(Path::new(&args.relative_path))?;

    match fs::read_dir(&safe_path) {
        Ok(entries) => {
            let result: Vec<String> = entries
                .flatten()
                .map(|entry| {
                    let mut name = entry.file_name().to_string_lossy().to_string();
                    if let Ok(file_meta) = entry.metadata()
                        && file_meta.is_dir()
                    {
                        name.push('/');
                    }
                    name
                })
                .collect();
            Ok(result)
        }
        Err(e) => Err(Error::Tool(format!("Error: failed to read dir: {e}"))),
    }
}
