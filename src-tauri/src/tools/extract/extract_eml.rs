use std::path::Path;
use crate::Result;

pub fn extract_eml(_file_path: &Path, _output_dir: &Path) -> Result<Vec<String>> {
    // Dummy implementation
    Ok(vec!["dummy_eml_extraction.txt".to_string()])
}