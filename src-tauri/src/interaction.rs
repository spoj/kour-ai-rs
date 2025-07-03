use crate::chat::{OutgoingMessage, Content, IncomingMessage, ToolCall};

#[derive(Clone)]
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

impl Interaction {
    pub fn from_llm(incoming_message: IncomingMessage) -> Self {
        let tool_calls = incoming_message.tool_calls;
        let content = match incoming_message.content {
            crate::chat::IncomingContent::Text(t) => vec![Content::Text { text: t }],
            crate::chat::IncomingContent::Parts(contents) => contents,
            crate::chat::IncomingContent::None => vec![],
        };
        Interaction::LlmResponse {
            content,
            tool_calls,
        }
    }
    pub fn from_user(content: Vec<Content>) -> Self {
        Self::UserMessage { content }
    }
    pub fn to_llm(&self) -> Vec<OutgoingMessage> {
        match self {
            Interaction::LlmResponse {
                content,
                tool_calls,
            } => vec![OutgoingMessage {
                role: "assistant".to_string(),
                content: content.to_owned(),
                tool_calls: tool_calls.to_owned(),
                tool_call_id: None,
            }],
            Interaction::ToolResult {
                tool_call_id,
                response,
                #[allow(unused_variables)]
                for_llm,
                #[allow(unused_variables)]
                for_user,
            } => vec![OutgoingMessage {
                role: "tool".to_string(),
                content: vec![Content::Text {
                    text: response.to_owned(),
                }],
                tool_calls: None,
                tool_call_id: Some(tool_call_id.to_owned()),
            }],
            Interaction::UserMessage { content } => vec![OutgoingMessage {
                role: "user".to_string(),
                content: content.to_owned(),
                tool_calls: None,
                tool_call_id: None,
            }],
        }
    }
    pub fn to_user(&self) -> Vec<OutgoingMessage> {
        match self {
            Interaction::LlmResponse {
                content,
                tool_calls,
            } => vec![OutgoingMessage {
                role: "assistant".to_string(),
                content: content.to_owned(),
                tool_calls: tool_calls.to_owned(),
                tool_call_id: None,
            }],
            Interaction::ToolResult {
                tool_call_id,
                response,
                #[allow(unused_variables)]
                for_llm,
                #[allow(unused_variables)]
                for_user,
            } => vec![OutgoingMessage {
                role: "tool".to_string(),
                content: vec![Content::Text {
                    text: response.to_owned(),
                }],
                tool_calls: None,
                tool_call_id: Some(tool_call_id.to_owned()),
            }],
            Interaction::UserMessage { content } => vec![OutgoingMessage {
                role: "user".to_string(),
                content: content.to_owned(),
                tool_calls: None,
                tool_call_id: None,
            }],
        }
    }
}
