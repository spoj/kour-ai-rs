use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{fs, task};

use crate::error::Error;
use crate::tools::{Function, Tool};
use crate::Result;

pub mod extract_eml;
pub mod extract_msg;
pub mod extract_zip;

#[derive(Serialize, Deserialize)]
pub struct ExtractArgs {
    pub filename: String,
}

#[derive(Serialize)]
pub struct ExtractResult {
    pub status: String,
    pub extraction_folder: String,
    pub extracted_files: Vec<String>,
    pub total_files: usize,
}

pub async fn extract(args: ExtractArgs) -> Result<ExtractResult> {
    let root_dir = task::spawn_blocking(crate::get_settings_fn)
        .await?
        .map(|s| s.root_dir)?;
    let root_dir = PathBuf::from(root_dir);

    let file_path = root_dir.join(&args.filename);

    if !file_path.is_file() {
        return Err(Error::Tool("File not found".to_string()));
    }

    let extraction_folder = file_path.with_extension(format!(
        "{}.extracted",
        file_path.extension().unwrap_or_default().to_str().unwrap()
    ));

    fs::create_dir_all(&extraction_folder).await?;

    let file_extension = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let extracted_files = match file_extension.as_str() {
        "zip" => {
            let file_path = file_path.to_path_buf();
            let extraction_folder = extraction_folder.to_path_buf();
            tokio::task::spawn_blocking(move || {
                extract_zip::extract_zip(&file_path, &extraction_folder)
            })
            .await??
        }
        "eml" => {
            let file_path = file_path.to_path_buf();
            let extraction_folder = extraction_folder.to_path_buf();
            tokio::task::spawn_blocking(move || {
                extract_eml::extract_eml(&file_path, &extraction_folder)
            })
            .await??
        }
        "msg" => {
            let file_path = file_path.to_path_buf();
            let extraction_folder = extraction_folder.to_path_buf();
            tokio::task::spawn_blocking(move || {
                extract_msg::extract_msg(&file_path, &extraction_folder)
            })
            .await??
        }
        _ => {
            return Err(Error::Tool("Unsupported file type".to_string()))
        }
    };

    let result: ExtractResult = ExtractResult {
        status: "success".to_string(),
        extraction_folder: extraction_folder.to_str().unwrap().to_string(),
        total_files: extracted_files.len(),
        extracted_files,
    };

    Ok(result)
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "extract".to_string(),
            description: "Extract content from email files (.msg, .eml) and zip archives (.zip)."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The path to the file to extract."
                    }
                },
                "required": ["filename"]
            }),
        },
    }
}
