use std::{cmp::Ordering, convert};

use crate::{
    Result,
    interaction::Content,
    interaction::{Interaction, Source, Target},
};
use serde::Serialize;
use tauri::Emitter;

#[derive(Clone)]
pub struct UIEvents {
    window: tauri::Window,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub enum EventPayload<'a> {
    Start,
    End,
    Message {
        role: &'a str,
        content: &'a [Content],
    },
    ToolCall {
        tool_name: &'a str,
        tool_call_id: &'a str,
        tool_args: &'a str,
    },
    ToolDone {
        tool_call_id: &'a str,
        tool_result: &'a str,
    },
}

impl<'a> Target<'a> for UIEvents {
    type RenderType = EventPayload<'a>;
    fn convert(interaction: &'a Interaction) -> Vec<EventPayload<'a>> {
        match interaction {
            Interaction::LlmResponse {
                content,
                tool_calls,
            } => {
                let mut out = vec![];
                out.push(EventPayload::Message {
                    role: "assistant",
                    content,
                });
                if let Some(tool_calls) = tool_calls {
                    out.extend(tool_calls.iter().map(|t| EventPayload::ToolCall {
                        tool_name: &t.function.name,
                        tool_call_id: &t.id,
                        tool_args: &t.function.arguments,
                    }));
                }
                out
            }
            Interaction::ToolResult {
                tool_call_id,
                response,
                #[allow(unused_variables)]
                for_llm,
                #[allow(unused_variables)]
                for_user,
            } => vec![EventPayload::ToolDone {
                tool_call_id,
                tool_result: response,
            }],
            Interaction::UserMessage { content } => vec![EventPayload::Message {
                role: "user",
                content,
            }],
        }
    }
}

impl Source for UIEvents {
    type SendType = Vec<Content>;

    fn sends(data: Vec<Content>) -> Interaction {
        Interaction::UserMessage { content: data }
    }
}

impl UIEvents {
    pub fn new(window: tauri::Window) -> Self {
        Self { window }
    }

    pub fn emit_tool_call(&self, name: &str, id: &str, arguments: &str) -> Result<()> {
        self.window.emit(
            "chat_completion_update",
            EventPayload::ToolCall {
                tool_name: name,
                tool_call_id: id,
                tool_args: arguments,
            },
        )?;
        Ok(())
    }

    pub fn emit_tool_result(&self, id: &str, result: &str) -> Result<()> {
        self.window.emit(
            "chat_completion_update",
            EventPayload::ToolDone {
                tool_call_id: id,
                tool_result: result,
            },
        )?;
        Ok(())
    }
    pub fn emit_done(&self) -> Result<()> {
        self.window
            .emit("chat_completion_update", EventPayload::End)?;
        Ok(())
    }

    pub fn emit_start(&self) -> Result<()> {
        self.window
            .emit("chat_completion_update", EventPayload::Start)?;
        Ok(())
    }

    pub fn emit_interaction(&self, interaction: &Interaction) -> Result<()> {
        for payload in Self::render(&[interaction.to_owned()]) {
            let _ = self.window.emit("chat_completion_update", payload);
        }
        Ok(())
    }

    pub fn replay_interactions(&self, interactions: &[Interaction]) -> Result<()> {
        for i in interactions {
            let _ = self.emit_interaction(i);
        }
        Ok(())
    }
}
