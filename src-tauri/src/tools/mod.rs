mod ask_files;
mod check_online;
mod extract;
mod find;
mod load_file;
mod ls;
mod notes;
mod roll_dice;

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, to_string, to_value};
use std::sync::LazyLock;

use crate::{Result, interaction::Content, error::Error, interaction::Interaction};

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

#[derive(Debug)]
pub struct ToolPayload {
    pub response: Value,
    pub for_llm: Vec<Content>,
    pub for_user: Vec<Content>,
}

impl ToolPayload {
    fn from<T>(response: T) -> Result<Self>
    where
        T: Serialize,
    {
        Ok(Self {
            response: to_value(response)?,
            for_llm: vec![],
            for_user: vec![],
        })
    }
    fn llm(mut self, for_llm: Vec<Content>) -> Self {
        self.for_llm = for_llm;
        dbg!(&self);
        self
    }
    fn user(mut self, for_user: Vec<Content>) -> Self {
        self.for_user = for_user;
        self
    }
    pub fn finalize(self, tool_call_id: String) -> Result<Interaction> {
        Ok(Interaction::ToolResult {
            tool_call_id,
            response: to_string(&self.response).map_err(crate::error::Error::Json)?,
            for_llm: self.for_llm,
            for_user: self.for_user,
        })
    }
}

pub static TOOLS: LazyLock<Vec<Tool>> = LazyLock::new(|| {
    vec![
        roll_dice::get_tool(),
        ls::get_tool(),
        find::get_tool(),
        notes::read_notes_tool(),
        notes::append_notes_tool(),
        ask_files::get_tool(),
        extract::get_tool(),
        load_file::get_tool(),
        check_online::get_tool(),
    ]
});

pub async fn tool_executor(name: &str, arguments: &str) -> ToolPayload {
    async fn tool_executor_inner(name: &str, arguments: &str) -> crate::Result<ToolPayload> {
        let x = match name {
            "ls" => Ok(ls::ls(from_str(arguments)?).await?),
            "roll_dice" => Ok(roll_dice::execute(from_str(arguments)?).await),
            "find" => Ok(find::find(from_str(arguments)?).await?),
            "read_notes" => Ok(notes::read_notes().await?),
            "append_notes" => Ok(notes::append_notes(from_str(arguments)?).await?),
            "ask_files" => Ok(ask_files::ask_files(from_str(arguments)?).await?),
            "extract" => Ok(extract::extract(from_str(arguments)?).await?),
            "load_file" => Ok(load_file::load_file(from_str(arguments)?).await?),
            "check_online" => Ok(check_online::check_online(from_str(arguments)?).await?),
            _ => Err(Error::Tool("Tool Not Found".to_string())),
        };
        x
    }

    tool_executor_inner(name, arguments)
        .await
        .unwrap_or(ToolPayload {
            response: Value::String("Error".to_string()),
            for_llm: vec![],
            for_user: vec![],
        })
}
