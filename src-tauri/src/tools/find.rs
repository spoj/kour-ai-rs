use crate::{settings::get_root, Result};
use crate::error::Error;
use crate::utils::jailed::Jailed;

use super::{Function, Tool};
use glob::MatchOptions;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "find".to_string(),
            description: "Locates files by glob, returning up to 'list_max_files' matches. If more files match, it returns an error and the total count, prompting you to refine the glob. Excellent for targeted searches when you expect a manageable number of results. Use 'ls' to confirm existence or explore a directory before crafting a glob.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "glob": {
                        "type": "string",
                        "description": "The glob pattern to match against"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum results. If glob matches more than this, the tool will return an error to avoid overwhelming the user. Start with 200 and adjust approach if required."
                    }
                },
                "required": ["glob","max_results"]
            }),
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FindArgs {
    pub glob: String,
    pub max_results: usize,
}

pub fn find_internal(root_dir: &str, glob_pattern: &str, limit: usize) -> Result<Vec<String>> {
    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }
    let jail = Path::new(root_dir);
    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    // Use the Jailed trait to prevent traversal attacks.
    // We only join the non-glob part of the pattern to validate the base directory.
    let glob_path = Path::new(glob_pattern);
    let safe_base = jail.jailed_join(Path::new(glob_path.parent().unwrap_or(glob_path)))?;

    let full_pattern = safe_base.join(glob_path.file_name().unwrap_or_default());
    let full_pattern_str = full_pattern
        .to_str()
        .ok_or_else(|| Error::Tool("Invalid pattern path".to_string()))?;

    let entries: Vec<_> = glob::glob_with(full_pattern_str, options)
        .map_err(|e| Error::Tool(format!("Invalid glob pattern: {e}")))?
        .flatten()
        .collect();

    if entries.len() > limit {
        return Err(Error::Tool(format!(
            "Error: Found more files ({}) than limit ({limit}). Consider up the limit or take different approach.",
            entries.len()
        )));
    }

    let result: Vec<String> = entries
        .iter()
        .map(|entry| {
            if let Ok(relative) = entry.strip_prefix(root_dir) {
                relative.to_str().unwrap_or_default().to_string()
            } else {
                String::new()
            }
        })
        .collect();

    Ok(result)
}

pub async fn find(args: FindArgs) -> Result<Vec<String>> {
    let root_dir = get_root()?;
    let result = find_internal(&root_dir, &args.glob, args.max_results)?;
    Ok(result)
}
