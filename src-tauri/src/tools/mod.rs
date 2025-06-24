mod ls;
mod roll_dice;

use serde::{Deserialize, Serialize};
use serde_json::{from_str, Value};
use std::sync::LazyLock;

use crate::error::Error;

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

pub static TOOLS: LazyLock<Vec<Tool>> =
    LazyLock::new(|| vec![roll_dice::get_tool(), ls::get_tool()]);

pub async fn tool_executor(name: &str, arguments: &str) -> crate::Result<String> {
    match name {
        "roll_dice" => roll_dice::execute(roll_dice::RollDiceArgs {}).await,
        "ls" => {
            let args = from_str(arguments)?;
            Ok(ls::ls(args).await?)
        }
        _ => Err(Error::Tool("Tool Not Found".to_string())),
    }
}
