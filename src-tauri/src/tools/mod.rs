mod ask_files;
mod check_online;
mod extract;
mod find;
mod load_file;
mod ls;
mod make_file;
mod notes;
mod roll_dice;

use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, to_string, to_value};
use std::sync::LazyLock;

use crate::{
    Result,
    error::Error,
    interaction::{Content, Interaction},
};

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
    pub response: Result<Value>,
    pub for_llm: Vec<Content>,
    pub for_user: Vec<Content>,
}

trait ToolPayloadable {
    fn to_payload(self) -> ToolPayload;
}

impl<T> ToolPayloadable for Result<T>
where
    T: Serialize,
{
    fn to_payload(self) -> ToolPayload {
        ToolPayload::from(self)
    }
}

impl ToolPayloadable for ToolPayload {
    fn to_payload(self) -> ToolPayload {
        self
    }
}
impl ToolPayloadable for Result<ToolPayload> {
    fn to_payload(self) -> ToolPayload {
        match self {
            Ok(o) => o,
            Err(e) => ToolPayload::from::<Error>(Err(e)),
        }
    }
}

async fn tool_execute<'a, F, X, Y>(tool_fn: F, data: &'a str) -> ToolPayload
where
    F: AsyncFn(X) -> Y,
    X: Deserialize<'a>,
    Y: ToolPayloadable,
{
    let data = match data {
        "" => "{}",
        _ => data,
    };
    match from_str(data) {
        Ok(data) => tool_fn(data).await.to_payload(),
        Err(e) => ToolPayload::from::<Error>(Err(e.into())),
    }
}

impl ToolPayload {
    fn from<T>(response: Result<T>) -> Self
    where
        T: Serialize,
    {
        Self {
            response: match response {
                Ok(r) => to_value(r).map_err(|e| e.into()),
                Err(e) => Err(e),
            },
            for_llm: vec![],
            for_user: vec![],
        }
    }
    fn llm(mut self, for_llm: Vec<Content>) -> Self {
        self.for_llm = for_llm;
        dbg!(&self);
        self
    }
    #[allow(dead_code)]
    fn user(mut self, for_user: Vec<Content>) -> Self {
        self.for_user = for_user;
        self
    }
    pub fn finalize(self, tool_call_id: String) -> Interaction {
        Interaction::ToolResult {
            tool_call_id,
            response: to_string(&self.response)
                .unwrap_or("Error turning tool result to String".to_string()),
            for_llm: self.for_llm,
            for_user: self.for_user,
        }
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
        make_file::get_tool(),
        check_online::get_tool(),
    ]
});

pub async fn tool_dispatcher(name: &str, arguments: &str) -> ToolPayload {
    match name {
        "ls" => tool_execute(ls::ls, arguments).await,
        "roll_dice" => tool_execute(roll_dice::execute, arguments).await,
        "find" => tool_execute(find::find, arguments).await,
        "read_notes" => tool_execute(notes::read_notes, arguments).await,
        "append_notes" => tool_execute(notes::append_notes, arguments).await,
        "ask_files" => tool_execute(ask_files::ask_files, arguments).await,
        "extract" => tool_execute(extract::extract, arguments).await,
        "load_file" => tool_execute(load_file::load_file, arguments).await,
        "make_file" => tool_execute(make_file::make_file, arguments).await,
        "check_online" => tool_execute(check_online::check_online, arguments).await,
        _ => ToolPayload::from::<Error>(Err(Error::Tool("no such tool".to_string()))),
    }
}
