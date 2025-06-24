use crate::error::Error;
use crate::utils::jailed::Jailed;
use crate::Result;

use super::{Function, Tool};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::task;

const MAX_FIND_RESULTS: usize = 100;

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

fn find_internal(root_dir: &str, glob_pattern: &str) -> Result<String> {
    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }
    let jail = Path::new(root_dir);

    // Use the Jailed trait to prevent traversal attacks.
    // We only join the non-glob part of the pattern to validate the base directory.
    let glob_path = Path::new(glob_pattern);
    let safe_base = jail.jailed_join(Path::new(glob_path.parent().unwrap_or(glob_path)))?;

    let full_pattern = safe_base.join(glob_path.file_name().unwrap_or_default());
    let full_pattern_str = full_pattern
        .to_str()
        .ok_or_else(|| Error::Tool("Invalid pattern path".to_string()))?;

    let entries: Vec<_> = glob::glob(full_pattern_str)
        .map_err(|e| Error::Tool(format!("Invalid glob pattern: {}", e)))?
        .flatten()
        .collect();

    if entries.len() > MAX_FIND_RESULTS {
        return Err(Error::Tool(format!(
            "Error: Find returned too many results ({}). Please provide a more specific glob pattern.",
            entries.len()
        )));
    }

    let result: Vec<String> = entries
        .iter()
        .map(|entry| {
            entry
                .to_str()
                .unwrap_or_default()
                .strip_prefix(root_dir)
                .unwrap_or_default()
                .to_string()
        })
        .collect();

    Ok(result.join("\n"))
}

pub async fn find(args: FindArgs) -> Result<String> {
    let root_dir = task::spawn_blocking(crate::get_settings_fn)
        .await?
        .map(|s| s.root_dir)?;
    find_internal(&root_dir, &args.glob)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_safe_pattern() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), "content").unwrap();
        fs::write(dir.path().join("file2.log"), "content").unwrap();

        let root = dir.path().to_str().unwrap();
        let result = find_internal(root, "*.txt").unwrap();

        assert!(result.contains("file1.txt"));
        assert!(!result.contains("file2.log"));
    }

    #[test]
    fn test_find_in_subdirectory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.log"), "content").unwrap();

        let root = dir.path().to_str().unwrap();
        let result = find_internal(root, "subdir/*.log").unwrap();

        assert!(result.contains("subdir/file.log") || result.contains("subdir\\file.log"));
    }

    #[test]
    fn test_find_prevents_traversal() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("should_not_be_found.txt"), "secret").unwrap();

        let root = dir.path().to_str().unwrap();
        // This glob attempts to escape the temporary root directory.
        let result = find_internal(root, "../../*");

        assert!(result.is_err());
        if let Err(Error::Tool(msg)) = result {
            assert!(msg.contains("Path traversal attempt detected"));
        } else {
            panic!("Expected a tool error for path traversal");
        }
    }
}
