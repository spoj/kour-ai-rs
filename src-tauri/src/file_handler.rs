use crate::Result;
use crate::error::Error;
use crate::interaction::{Content, FileData, ImageUrl};
use base64::{Engine as _, engine::general_purpose};
use calamine::{Reader, open_workbook_auto_from_rs};
use camino::Utf8PathBuf;
use csv::Writer;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::iter::once;
use std::path::Path;

fn get_cache_path(file_buffer: &[u8], target_extension: &str) -> Result<Utf8PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(file_buffer);
    let result = hasher.finalize();
    let hash = format!("{result:x}");
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
    Text,
    Unsupported,
}

pub fn determine_file_type(path: &Path) -> FileType {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => FileType::Image("image/jpeg".to_string()),
        "png" => FileType::Image("image/png".to_string()),
        "pdf" => FileType::Pdf,
        "docx" => FileType::Docx,
        "pptx" => FileType::Pptx,
        "xlsx" => FileType::Xlsx,
        "txt" | "md" | "csv" | "json" | "xml" | "html" | "css" | "js" | "ts" | "jsx" | "tsx"
        | "py" | "rb" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "go" | "php" | "swift"
        | "kt" | "rs" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "log" | "sh" | "bat" => {
            FileType::Text
        }
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

    let soffice = crate::settings::get_settings()?.soffice_path;
    let temp_dir = Builder::new()
        .prefix("file_conversion")
        .tempdir()
        .map_err(|_| Error::Tool("File conversion error".to_string()))?;

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
    pdf_path.push(
        path.file_name()
            .ok_or(Error::Tool("PDF conversion error".to_string()))?,
    );
    pdf_path.set_extension("pdf");

    let pdf_bytes = std::fs::read(&pdf_path)?;

    write_conversion_cache(&file_buffer, &pdf_bytes, "pdf");
    std::fs::remove_file(&pdf_path)?;

    Ok(pdf_bytes)
}

fn convert_xlsx_to_csv(file_buffer: &[u8]) -> Result<String> {
    println!("starting to convert csv");
    if let Ok(Some(cached_csv_bytes)) = read_conversion_cache(file_buffer, "csv") {
        return Ok(String::from_utf8(cached_csv_bytes).unwrap_or_default());
    }
    println!("no cache");
    let reader = Cursor::new(file_buffer);
    let mut workbook = open_workbook_auto_from_rs(reader)
        .map_err(|_| Error::Tool("cannot open xlsx workbook".to_string()))?;
    println!("wb opened");
    let mut csv_data = vec![];
    for (sheet_name, range) in workbook.worksheets() {
        let mut writer = Writer::from_writer(vec![]);
        println!("enter a sheet");
        for row in range.rows() {
            writer
                .write_record(once(sheet_name.clone()).chain(row.iter().map(|c| c.to_string())))
                .map_err(|_| Error::Tool("error writing CSV".to_string()))?;
        }
        csv_data.extend(
            writer
                .into_inner()
                .map_err(|_| Error::Tool("CSV conversion error".to_string()))?,
        );
    }
    println!("done all sheets");

    write_conversion_cache(file_buffer, &csv_data, "csv");
    println!("written to cache");
    String::from_utf8(csv_data).map_err(|_| Error::Tool("error converting CSV to UTF".to_string()))
}

pub fn process_file_for_llm(path: &Path) -> Result<Vec<Content>> {
    let file_buffer = fs::read(path)?;
    let file_type = determine_file_type(path);

    match file_type {
        FileType::Image(mime) => {
            let encoded = general_purpose::STANDARD.encode(&file_buffer);
            let data_url = format!("data:{mime};base64,{encoded}");
            Ok(vec![Content::ImageUrl {
                image_url: ImageUrl { url: data_url },
            }])
        }
        FileType::Pdf => {
            let encoded = general_purpose::STANDARD.encode(&file_buffer);
            let data_url = format!("data:application/pdf;base64,{encoded}");
            Ok(vec![Content::File {
                file: FileData {
                    filename: path
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                    file_data: data_url,
                },
            }])
        }
        FileType::Docx | FileType::Pptx => {
            let pdf_bytes = convert_to_pdf(path)?;
            let encoded = general_purpose::STANDARD.encode(&pdf_bytes);
            let data_url = format!("data:application/pdf;base64,{encoded}");
            Ok(vec![Content::File {
                file: FileData {
                    filename: path
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                    file_data: data_url,
                },
            }])
        }
        FileType::Xlsx => {
            let csv_data = convert_xlsx_to_csv(&file_buffer)?;
            Ok(vec![Content::Text { text: csv_data }])
        }
        FileType::Text => {
            let content = fs::read_to_string(path)?;
            Ok(vec![Content::Text { text: content }])
        }
        _ => Err(Error::Tool("Unsupported file type".to_string())),
    }
}
