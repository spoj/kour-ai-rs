use crate::error::Error;
use chrono::Local;
use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tokio::task;

use crate::tools::Function;
use crate::tools::Tool;

pub async fn read_notes() -> crate::Result<String> {
    let notes_path = get_notes_path().await?;
    if !notes_path.exists() {
        return Ok("No notes found.".to_string());
    }
    fs::read_to_string(notes_path).map_err(|e| Error::Tool(e.to_string()))
}

#[derive(Deserialize)]
pub struct AppendNotesArgs {
    pub markdown_content: String,
}

pub async fn append_notes(args: AppendNotesArgs) -> crate::Result<String> {
    let notes_path = get_notes_path().await?;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let note_entry = format!(
        "<note date=\"{}\">\n{}\n</note>\n\n",
        timestamp, args.markdown_content
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(notes_path)
        .map_err(|e| Error::Tool(e.to_string()))?;

    file.write_all(note_entry.as_bytes())
        .map_err(|e| Error::Tool(e.to_string()))?;

    Ok("Note appended successfully.".to_string())
}

async fn get_notes_path() -> crate::Result<PathBuf> {
    let root_dir = task::spawn_blocking(crate::get_settings_fn)
        .await?
        .map(|s| s.root_dir)?;

    if root_dir.is_empty() {
        return Err(Error::Tool(
            "Error: Root directory is not set. Please set it in the settings.".to_string(),
        ));
    }

    Ok(PathBuf::from(root_dir).join("_NOTES.txt"))
}

pub fn read_notes_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "read_notes".to_string(),
            description: "Reads all notes from the _NOTES.txt file.".to_string(),
            parameters: serde_json::Value::Object(serde_json::Map::new()),
        },
    }
}

pub fn append_notes_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "append_notes".to_string(),
            description: "Appends a markdown string to the _NOTES.txt file.".to_string(),
            parameters: serde_json::from_str(
                r#"{
                    "type": "object",
                    "properties": {
                        "markdown_content": {
                            "type": "string",
                            "description": "The markdown content to append to the notes."
                        }
                    },
                    "required": ["markdown_content"]
                }"#,
            )
            .unwrap(),
        },
    }
}
