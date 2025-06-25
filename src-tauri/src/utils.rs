use crate::error::Error;
use crate::Result;
use std::path::{Path, PathBuf};

pub fn get_safe_path(root_dir: &str, relative_path: &str) -> Result<PathBuf> {
    let root_dir_path = Path::new(root_dir);
    let path = root_dir_path.join(relative_path);

    let canonical_root = root_dir_path
        .canonicalize()
        .map_err(|_| Error::Tool("Failed to canonicalize the root dir".to_string()))?;

    let canonical_path = path
        .canonicalize()
        .map_err(|_| Error::Tool(format!("Failed to canonicalize path: {}", path.display())))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(Error::Tool("Error: path outside root dir".to_string()));
    }

    Ok(canonical_path)
}
