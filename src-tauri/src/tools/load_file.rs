use crate::Result;
use crate::file_handler;
use crate::interaction::Content;
use crate::settings::get_root;
use crate::tools::{Function, Tool, ToolPayload};
use crate::utils::jailed::Jailed;
use serde::Deserialize;
use std::path::Path;
use tokio::task;

#[derive(Deserialize)]
pub struct LoadFileArgs {
    pub filename: String,
}

pub async fn load_file(args: LoadFileArgs) -> Result<ToolPayload> {
    let root_dir = get_root()?;

    let safe_path = root_dir.jailed_join(Path::new(&args.filename))?;

    let mut file_content =
        task::spawn_blocking(move || file_handler::process_file_for_llm(&safe_path)).await??;

    // Prepend an instructional message for the LLM.
    let instructional_text = Content::Text {
        text: format!(
            "The content of the file '{}' has been loaded. Here is the full content for your context. Please use this content to answer any subsequent questions.",
            &args.filename
        ),
    };
    file_content.insert(0, instructional_text);

    Ok(ToolPayload::from(Ok("file_loaded".to_string())).llm(file_content))
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "load_file".to_string(),
            description:
                "Loads a file's entire content into the active context, enabling a comprehensive, holistic review. This is the only way to analyze how different parts of a document relate to each other and uncover nuanced insights.\n\n**Trade-off:** This method is more thorough and may take more processing time than a simple `ask_files` query. It is the required tool for strategic analysis, root cause investigation, and any task that requires understanding the full story behind the numbers."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The path to the file to load."
                    }
                },
                "required": ["filename"]
            }),
        },
    }
}
