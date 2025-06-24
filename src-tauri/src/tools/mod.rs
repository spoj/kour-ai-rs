mod roll_dice;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub static TOOLS: LazyLock<Vec<Tool>> = LazyLock::new(|| vec![roll_dice::get_tool()]);

pub async fn tool_executor(name: String, arguments: String) -> String {
    match name.as_str() {
        "roll_dice" => roll_dice::execute(arguments).await,
        _ => "Tool not found".to_string(),
    }
}
