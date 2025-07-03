use serde::{Deserialize, Serialize};

use crate::{chat::Content, openrouter::ToolCall};

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

pub trait Target<'a> {
    type RenderType: 'a;
    fn render(interactions: &'a [Interaction]) -> Vec<Self::RenderType>;
}
pub trait Source {
    type SendType;
    fn sends(data: Self::SendType) -> Interaction;
}
