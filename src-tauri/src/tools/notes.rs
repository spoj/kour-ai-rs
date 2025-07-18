use crate::error::Error;
use crate::settings::get_root;
use chrono::Local;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::tools::Function;
use crate::tools::Tool;

pub async fn read_notes(_args: Value) -> crate::Result<String> {
    let notes_path = get_notes_path().await?;
    if !notes_path.exists() {
        return Err(Error::Tool("file not found".to_string()));
    }
    let result = fs::read_to_string(notes_path).map_err(|e| Error::Tool(e.to_string()))?;
    Ok(result)
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
    let root_dir = get_root()?;

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
            description: "Reads the content of the `_NOTES.txt` file from the root directory. Use this to recall previous findings or context. Do NOT rely on read_notes for factuality or comprehensiveness. Only treat it as additional pools of direction to explore. This is because notes maybe stale (as the knowledge pool was updated) or that the notes are simplified for current user query. Everything that you answer users MUST be coming from querying primary documents, and NOT solely from your previous notes.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    }
}

pub fn append_notes_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "append_notes".to_string(),
            description: "Appends a new markdown entry to the `_NOTES.md` file. Use this to record significant learnings, complex file structures, interrelations between files, or user instructions for future reference. Each entry is automatically timestamped. Rule of thumb: if it takes more than 4 tool calls for your to discover something, it's worth noting down.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "markdown_content": {
                        "type": "string",
                        "description": "The markdown content to append to the notes."
                    }
                },
                "required": ["markdown_content"]
            }),
        },
    }
}
