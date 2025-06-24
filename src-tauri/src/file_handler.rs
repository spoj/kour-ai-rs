use crate::chat::{Content, FileData, ImageUrl};
use crate::error::Error;
use crate::Result;
use base64::{engine::general_purpose, Engine as _};
use calamine::{open_workbook, Reader, Xlsx};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn get_cache_path(file_buffer: &[u8], target_extension: &str) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(file_buffer);
    let result = hasher.finalize();
    let hash = format!("{:x}", result);
    let cache_dir = crate::get_cache_dir()?;
    let sub_dir = &hash[0..2];
    let cache_file_name = format!("{}.{}", &hash[2..], target_extension);
    Ok(cache_dir.join(sub_dir).join(cache_file_name))
}

fn read_conversion_cache(file_buffer: &[u8], target_extension: &str) -> Result<Option<Vec<u8>>> {
    let cache_path = get_cache_path(file_buffer, target_extension)?;
    if cache_path.exists() {
        Ok(fs::read(cache_path).ok())
    } else {
        Ok(None)
    }
}

pub fn get_cache(
    orig_file_buffer: &[u8],
    _orig_ext: &str,
    target_ext: &str,
) -> Result<Option<Vec<u8>>> {
    read_conversion_cache(orig_file_buffer, target_ext)
}

fn write_conversion_cache(
    orig_file_buffer: &[u8],
    converted_file_buf: &[u8],
    target_extension: &str,
) {
    if let Ok(cache_path) = get_cache_path(orig_file_buffer, target_extension) {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(cache_path, converted_file_buf).ok();
    }
}

pub enum FileType {
    Image(String),
    Pdf,
    Docx,
    Pptx,
    Xlsx,
    Text(String),
    Unsupported,
}

pub fn determine_file_type(path: &Path) -> FileType {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match extension {
        "jpg" | "jpeg" => FileType::Image("image/jpeg".to_string()),
        "png" => FileType::Image("image/png".to_string()),
        "pdf" => FileType::Pdf,
        "docx" => FileType::Docx,
        "pptx" => FileType::Pptx,
        "xlsx" => FileType::Xlsx,
        "txt" | "md" | "csv" => FileType::Text("text/plain".to_string()),
        _ => FileType::Unsupported,
    }
}

use std::process::Command;
use tempfile::Builder;

pub fn convert_to_pdf(path: &Path) -> Result<Vec<u8>> {
    let file_buffer = fs::read(path)?;
    if let Ok(Some(cached_pdf)) = read_conversion_cache(&file_buffer, "pdf") {
        return Ok(cached_pdf);
    }

    let soffice = crate::get_settings_fn()?.soffice_path;
    let temp_dir = Builder::new()
        .prefix("file_conversion")
        .tempdir()
        .map_err(Error::Io)?;

    let temp_dir_path = temp_dir.path();

    let output = Command::new(soffice)
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(temp_dir_path)
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Err(Error::Tool(format!(
            "LibreOffice conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let mut pdf_path = temp_dir_path.to_path_buf();
    pdf_path.push(path.file_name().unwrap());
    pdf_path.set_extension("pdf");

    let pdf_bytes = std::fs::read(&pdf_path)?;

    write_conversion_cache(&file_buffer, &pdf_bytes, "pdf");
    std::fs::remove_file(&pdf_path)?;

    Ok(pdf_bytes)
}

pub fn process_file_for_llm(path: &Path) -> Result<Vec<Content>> {
    let file_buffer = fs::read(path)?;
    let file_type = determine_file_type(path);

    let result = match file_type {
        FileType::Image(mime) => {
            let encoded = general_purpose::STANDARD.encode(&file_buffer);
            let data_url = format!("data:{};base64,{}", mime, encoded);
            Ok(vec![Content::ImageUrl {
                image_url: ImageUrl { url: data_url },
            }])
        }
        FileType::Pdf => {
            let encoded = general_purpose::STANDARD.encode(&file_buffer);
            let data_url = format!("data:application/pdf;base64,{}", encoded);
            Ok(vec![Content::File {
                file: FileData {
                    filename: path.file_name().unwrap().to_str().unwrap().to_string(),
                    file_data: data_url,
                },
            }])
        }
        FileType::Docx | FileType::Pptx => {
            let pdf_bytes = convert_to_pdf(path)?;
            let encoded = general_purpose::STANDARD.encode(&pdf_bytes);
            let data_url = format!("data:application/pdf;base64,{}", encoded);
            Ok(vec![Content::File {
                file: FileData {
                    filename: path.file_name().unwrap().to_str().unwrap().to_string(),
                    file_data: data_url,
                },
            }])
        }
        FileType::Xlsx => {
            if let Ok(Some(cached_csv_bytes)) = read_conversion_cache(&file_buffer, "csv") {
                let csv_data = String::from_utf8(cached_csv_bytes).unwrap_or_default();
                return Ok(vec![Content::Text { text: csv_data }]);
            }

            let mut workbook: Xlsx<_> = open_workbook(path).unwrap();
            let mut csv_data = String::new();
            if let Ok(range) = workbook.worksheet_range("Sheet1") {
                for row in range.rows() {
                    let line = row
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    csv_data.push_str(&line);
                    csv_data.push('\n');
                }
            }
            write_conversion_cache(&file_buffer, csv_data.as_bytes(), "csv");

            Ok(vec![Content::Text { text: csv_data }])
        }
        FileType::Text(_) => {
            let content = fs::read_to_string(path)?;
            Ok(vec![Content::Text { text: content }])
        }
        _ => Err(Error::Tool("Unsupported file type".to_string())),
    };

    result
}
