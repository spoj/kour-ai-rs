mod find;
mod ls;
mod map_query;
mod notes;
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

pub static TOOLS: LazyLock<Vec<Tool>> = LazyLock::new(|| {
    vec![
        roll_dice::get_tool(),
        ls::get_tool(),
        find::get_tool(),
        notes::read_notes_tool(),
        notes::append_notes_tool(),
        map_query::get_tool(),
    ]
});

pub async fn tool_executor(name: &str, arguments: &str) -> crate::Result<String> {
    match name {
        "roll_dice" => roll_dice::execute(roll_dice::RollDiceArgs {}).await,
        "ls" => Ok(ls::ls(from_str(arguments)?).await?),
        "find" => Ok(find::find(from_str(arguments)?).await?),
        "read_notes" => notes::read_notes().await,
        "append_notes" => notes::append_notes(from_str(arguments)?).await,
        "map_query" => map_query::map_query(from_str(arguments)?).await,
        _ => Err(Error::Tool("Tool Not Found".to_string())),
    }
}
