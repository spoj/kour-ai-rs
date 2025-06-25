use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::tools::{Function, Tool};
use crate::Result;

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

use crate::error::Error;

use std::fs as std_fs;
use std::io;

fn extract_zip(file_path: &Path, output_dir: &Path) -> Result<Vec<String>> {
    let file = std_fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| Error::Tool(format!("Failed to read zip archive: {}", e)))?;
    let mut extracted_files = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| Error::Tool(e.to_string()))?;
        let outpath = match file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            std_fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std_fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std_fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        if let Some(path_str) = outpath.to_str() {
            extracted_files.push(path_str.to_string());
        }
    }
    Ok(extracted_files)
}

use mail_parser::{MessageParser, MimeHeaders};

fn extract_email(file_path: &Path, output_dir: &Path) -> Result<Vec<String>> {
    let content = std_fs::read(file_path)?;
    let message = MessageParser::default()
        .parse(&content)
        .ok_or(Error::Tool("Mail parsing error".to_string()))?;
    let mut extracted_files = Vec::new();

    let mut markdown_content = String::new();

    if let Some(from) = message.from() {
        markdown_content.push_str(&format!(
            "**From:** {}\n\n",
            from.first().unwrap().name().unwrap()
        ));
    }

    markdown_content.push_str("---\n\n");

    if let Some(html_body) = message.body_html(0) {
        markdown_content.push_str(html_body.as_ref());
    } else if let Some(text_body) = message.body_text(0) {
        markdown_content.push_str(text_body.as_ref());
    }

    let email_md_path = output_dir.join("EMAIL.md");
    std_fs::write(&email_md_path, markdown_content)?;
    extracted_files.push(email_md_path.to_str().unwrap().to_string());

    for attachment in message.attachments() {
        let filename = attachment.attachment_name().unwrap_or("unnamed_attachment");
        let attachment_path = output_dir.join(filename);
        std_fs::write(&attachment_path, attachment.contents())?;
        extracted_files.push(attachment_path.to_str().unwrap().to_string());
    }

    Ok(extracted_files)
}

pub async fn extract(args: &ExtractArgs) -> Result<String> {
    let root_dir = PathBuf::from("./"); // Assuming root is current dir for now
    let file_path = root_dir.join(&args.filename);

    if !file_path.is_file() {
        return Ok(json!({ "error": format!("File not found: {}", args.filename) }).to_string());
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
            tokio::task::spawn_blocking(move || extract_zip(&file_path, &extraction_folder))
                .await??
        }
        "eml" | "msg" => {
            let file_path = file_path.to_path_buf();
            let extraction_folder = extraction_folder.to_path_buf();
            tokio::task::spawn_blocking(move || extract_email(&file_path, &extraction_folder))
                .await??
        }
        _ => {
            return Ok(json!({
                "error": format!("Unsupported file type for extraction: {}. Supported types: .zip, .eml, .msg", file_extension)
            })
            .to_string())
        }
    };

    let result = ExtractResult {
        status: "success".to_string(),
        extraction_folder: extraction_folder.to_str().unwrap().to_string(),
        total_files: extracted_files.len(),
        extracted_files,
    };

    Ok(serde_json::to_string(&result)?)
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
