use serde::{Deserialize, Serialize};

use crate::openrouter::ToolCall;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
    #[serde(rename = "file")]
    File { file: FileData },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct FileData {
    pub filename: String,
    pub file_data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct ImageUrl {
    pub url: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Interaction {
    LlmResponse {
        content: Vec<Content>,
        tool_calls: Option<Vec<ToolCall>>,
    },
    ToolResult {
        tool_call_id: String,
        response: String,
        for_llm: Vec<Content>,
        for_user: Vec<Content>,
    },
    UserMessage {
        content: Vec<Content>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct History {
    pub inner: Vec<Interaction>,
}

impl History {
    pub fn push(&mut self, new: Interaction) {
        self.inner.push(new);
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

pub trait Target<'a> {
    type RenderType: 'a;

    fn convert(interactions: &'a Interaction) -> Vec<Self::RenderType>;
    fn render(history: &'a History) -> Vec<Self::RenderType> {
        history
            .inner
            .iter()
            .flat_map(|i| Self::convert(i))
            .collect()
    }
}
pub trait Source {
    type SendType;
    fn sends(data: Self::SendType) -> Interaction;
}
