use crate::interaction::{Content, FileData};
use crate::tools::{Function, Tool, ToolPayload};
use base64::Engine;
use base64::engine::general_purpose;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MakeFileArgs {
    pub content: String,
}

pub async fn make_file(args: MakeFileArgs) -> ToolPayload {
    let bytes = args.content.as_bytes();
    let encoded = general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:text/plain;base64,{encoded}");

    let content = vec![Content::File {
        file: FileData {
            filename: "file.txt".to_string(),
            file_data: data_url,
        },
    }];
    ToolPayload::from(Ok("Created file".to_string())).user(content)
}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "make_file".to_string(),
            description: "Creates a file which is then made available to user".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The content to be included in the file"
                    }
                },
                "required": ["content"]
            }),
        },
    }
}
