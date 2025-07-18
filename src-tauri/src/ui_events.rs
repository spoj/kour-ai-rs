use crate::{
    Result,
    interaction::{Content, History, Interaction, Source, Target},
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
        id: usize,
        role: &'a str,
        content: &'a [Content],
    },
    ToolCall {
        id: usize,
        tool_name: &'a str,
        tool_call_id: &'a str,
        tool_args: &'a str,
    },
    ToolDone {
        id: usize,
        tool_call_id: &'a str,
        tool_result: &'a str,
    },
}

impl<'a> Target<'a> for UIEvents {
    type RenderType = EventPayload<'a>;
    fn convert(interaction: &'a Interaction) -> Vec<EventPayload<'a>> {
        match interaction {
            Interaction::LlmResponse {
                interaction_id: id,
                content,
                tool_calls,
            } => {
                let mut out = vec![];
                if let Some(tool_calls) = tool_calls {
                    out.extend(tool_calls.iter().map(|t| EventPayload::ToolCall {
                        id: *id,
                        tool_name: &t.function.name,
                        tool_call_id: &t.id,
                        tool_args: &t.function.arguments,
                    }));
                } else {
                    out.push(EventPayload::Message {
                        id: *id,
                        role: "assistant",
                        content,
                    });
                }
                out
            }
            Interaction::ToolResult {
                interaction_id: id,
                tool_call_id,
                response,
                for_user,
                ..
            } => {
                let mut out = vec![EventPayload::ToolDone {
                    id: *id,
                    tool_call_id,
                    tool_result: response,
                }];
                if !for_user.is_empty() {
                    out.push(EventPayload::Message {
                        id: *id,
                        role: "assistant",
                        content: for_user,
                    });
                }
                out
            }
            Interaction::UserMessage {
                interaction_id: id,
                content,
            } => vec![EventPayload::Message {
                id: *id,
                role: "user",
                content,
            }],
        }
    }
}

impl Source for UIEvents {
    type SendType = Vec<Content>;

    fn sends(data: Vec<Content>) -> Interaction {
        Interaction::user_message(data)
    }
}

impl UIEvents {
    pub fn new(window: tauri::Window) -> Self {
        Self { window }
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
        for payload in Self::convert(interaction) {
            let _ = self.window.emit("chat_completion_update", payload);
        }
        Ok(())
    }

    pub fn replay_history(&self, history: &History) -> Result<()> {
        for payload in Self::render(history) {
            let _ = self.window.emit("chat_completion_update", payload);
        }
        Ok(())
    }
}
