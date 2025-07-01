use std::fs as std_fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::Result;
use crate::error::Error;

pub fn extract_zip(file_path: &Path, output_dir: &Path) -> Result<Vec<String>> {
    let source = std_fs::File::open(file_path)?;
    zip_extract::extract(source, output_dir, true)
        .map_err(|e| Error::Tool(format!("Failed to extract zip archive: {e}")))?;

    let mut extracted_files = Vec::new();
    for entry in WalkDir::new(output_dir) {
        if let Ok(entry) = entry
            && entry.file_type().is_file()
            && let Some(path_str) = entry.path().to_str()
        {
            extracted_files.push(path_str.to_string());
        }
    }

    Ok(extracted_files)
}
