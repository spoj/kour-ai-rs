mod extract;
mod find;
mod ls;
mod ask_files;
mod notes;
mod roll_dice;
mod load_file;
mod check_online;

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
        // roll_dice::get_tool(),
        ls::get_tool(),
        // find::get_tool(),
        // notes::read_notes_tool(),
        // notes::append_notes_tool(),
        // ask_files::get_tool(),
        // extract::get_tool(),
        // load_file::get_tool(),
        // check_online::get_tool(),
    ]
});

pub async fn tool_executor(name: &str, arguments: &str) -> crate::Result<Value> {
    match name {
        "roll_dice" => Ok(serde_json::to_value(roll_dice::execute(from_str(arguments)?).await)?),
        "ls" => Ok(serde_json::to_value(ls::ls(from_str(arguments)?).await?)?),
        "find" => Ok(serde_json::to_value(find::find(from_str(arguments)?).await?)?),
        "read_notes" => Ok(serde_json::to_value(notes::read_notes().await?)?),
        "append_notes" => Ok(serde_json::to_value(notes::append_notes(from_str(arguments)?).await?)?),
        "ask_files" => Ok(serde_json::to_value(ask_files::ask_files(from_str(arguments)?).await?)?),
        "extract" => Ok(serde_json::to_value(extract::extract(from_str(arguments)?).await?)?),
        // "load_file" => Ok(serde_json::to_value(load_file::load_file(from_str(arguments)?).await?)?),
        // "check_online" => {
        //     Ok(serde_json::to_value(check_online::check_online(from_str(arguments)?).await?)?)
        // }
        _ => Err(Error::Tool("Tool Not Found".to_string())),
    }
}
