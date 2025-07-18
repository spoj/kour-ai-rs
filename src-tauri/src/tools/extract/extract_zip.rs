use std::fs as std_fs;
use std::path::Path;

use crate::Result;
use crate::error::Error;

pub fn extract_zip(file_path: &Path, output_dir: &Path) -> Result<Vec<String>> {
    let source = std_fs::File::open(file_path)?;
    let mut zip =
        zip::ZipArchive::new(source).map_err(|_| Error::Tool("cannot extract".to_string()))?;
    zip.extract(output_dir)
        .map_err(|_| Error::Tool("cannot extract".to_string()))?;

    let mut extracted_files = Vec::new();
    for entry in ignore::Walk::new(output_dir) {
        if let Ok(entry) = entry
            && let Some(file_type) = entry.file_type()
            && file_type.is_file()
            && let Some(path_str) = entry.path().to_str()
        {
            extracted_files.push(path_str.to_string());
        }
    }

    Ok(extracted_files)
}
