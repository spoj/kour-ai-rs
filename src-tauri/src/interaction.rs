use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;

use crate::chat::{Content, IncomingContent, IncomingMessage, ToolCall};

#[derive(Clone, Serialize, Deserialize)]
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

pub trait Interactor {
    type RenderType;
    type SendType;
    fn render(interactions: &[Interaction]) -> Vec<Self::RenderType>;
    fn sends(data: Self::SendType) -> Interaction;
}

pub struct User;
pub struct Llm;

impl Interactor for Llm {
    type RenderType = Value;
    type SendType = IncomingMessage;
    fn render(interactions: &[Interaction]) -> Vec<Value> {
        interactions
            .iter()
            .flat_map(|i| match i {
                Interaction::LlmResponse {
                    content,
                    tool_calls,
                } => {
                    if let Some(tool_calls) = tool_calls {
                        vec![json!({
                    "role": "assistant",
                    "tool_calls": tool_calls})]
                    } else {
                        vec![json!({
                    "role": "assistant",
                    "content": content})]
                    }
                }
                Interaction::ToolResult {
                    tool_call_id,
                    response,
                    #[allow(unused_variables)]
                    for_llm,
                    #[allow(unused_variables)]
                    for_user,
                } => vec![json!({
                "role": "tool",
                "content": response,
                "tool_call_id": tool_call_id,
                })],
                Interaction::UserMessage { content } => vec![json!({
                        "role": "user",
                        "content": content,
                })],
            })
            .collect()
    }

    fn sends(data: IncomingMessage) -> Interaction {
        let tool_calls = data.tool_calls;
        let content = match data.content {
            IncomingContent::Text(t) => vec![Content::Text { text: t }],
            IncomingContent::Parts(contents) => contents,
            IncomingContent::None => vec![],
        };
        Interaction::LlmResponse {
            content,
            tool_calls,
        }
    }
}
impl Interactor for User {
    type RenderType = Value;
    type SendType = Vec<Content>;
    fn render(interactions: &[Interaction]) -> Vec<Value> {
        interactions
            .iter()
            .flat_map(|i| match i {
                Interaction::LlmResponse {
                    content,
                    tool_calls,
                } => vec![json!({
                    "role": "assistant",
                    "content": content,
                    "tool_calls": tool_calls})],
                Interaction::ToolResult {
                    tool_call_id,
                    response,
                    #[allow(unused_variables)]
                    for_llm,
                    #[allow(unused_variables)]
                    for_user,
                } => vec![json!({
                "role": "tool",
                "content": response,
                "tool_call_id": tool_call_id,
                })],
                Interaction::UserMessage { content } => vec![json!({
                        "role": "user",
                        "content": content,
                })],
            })
            .collect()
    }

    fn sends(data: Vec<Content>) -> Interaction {
        Interaction::UserMessage { content: data }
    }
}
