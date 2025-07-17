use std::{
    collections::HashSet,
    sync::atomic::{AtomicUsize, Ordering},
};

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

impl Content {
    fn is_empty(&self) -> bool {
        match self {
            Content::Text { text } => text.is_empty(),
            Content::ImageUrl { .. } => false,
            Content::File { .. } => false,
        }
    }
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

#[derive(Clone, Debug)]
pub enum Interaction {
    LlmResponse {
        interaction_id: usize,
        content: Vec<Content>,
        tool_calls: Option<Vec<ToolCall>>,
    },
    ToolResult {
        interaction_id: usize,
        tool_call_id: String,
        response: String,
        for_llm: Vec<Content>,
        for_user: Vec<Content>,
    },
    UserMessage {
        interaction_id: usize,
        content: Vec<Content>,
    },
}

impl Interaction {
    fn create_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn llm_response(content: Vec<Content>, tool_calls: Option<Vec<ToolCall>>) -> Interaction {
        Interaction::LlmResponse {
            content,
            tool_calls,
            interaction_id: Self::create_id(),
        }
    }
    pub fn tool_result(
        tool_call_id: String,
        response: String,
        for_llm: Vec<Content>,
        for_user: Vec<Content>,
    ) -> Interaction {
        Interaction::ToolResult {
            interaction_id: Self::create_id(),
            tool_call_id,
            response,
            for_llm,
            for_user,
        }
    }
    pub fn user_message(content: Vec<Content>) -> Interaction {
        Interaction::UserMessage {
            interaction_id: Self::create_id(),
            content,
        }
    }
    fn id(&self) -> usize {
        match self {
            Interaction::LlmResponse { interaction_id, .. } => *interaction_id,
            Interaction::ToolResult { interaction_id, .. } => *interaction_id,
            Interaction::UserMessage { interaction_id, .. } => *interaction_id,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Interaction::LlmResponse {
                content,
                tool_calls,
                ..
            } => {
                content.iter().all(|c| c.is_empty())
                    && tool_calls.as_ref().is_none_or(|c| c.is_empty())
            }
            Interaction::ToolResult { .. } => false,
            Interaction::UserMessage { content, .. } => content.iter().all(|c| c.is_empty()),
        }
    }
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
    pub fn clean_unfinished_tool_calls(&mut self) {
        let tool_response_ids: HashSet<_> = self
            .inner
            .iter()
            .flat_map(|i| {
                if let Interaction::ToolResult { tool_call_id, .. } = i {
                    Some(tool_call_id)
                } else {
                    None
                }
            })
            .cloned()
            .collect();
        if let Some(Interaction::LlmResponse { tool_calls, .. }) = self
            .inner
            .iter_mut()
            .rev()
            .find(|x| matches!(x, Interaction::LlmResponse { .. }))
            && let Some(tool_calls) = tool_calls
        {
            tool_calls.retain(|t| tool_response_ids.contains(&t.id));
        }
    }

    pub fn delete_by_tool_id(&mut self, llm_interaction_id: usize, tool_call_id_to_delete: &str) {
        let llm_interaction_index = self.inner.iter().position(|i| i.id() == llm_interaction_id);

        if let Some(index) = llm_interaction_index {
            // Remove the matching tool call from the LlmResponse
            if let Some(interaction) = self.inner.get_mut(index)
                && let Interaction::LlmResponse { tool_calls, .. } = interaction
                && let Some(calls) = tool_calls
            {
                calls.retain(|call| call.id != tool_call_id_to_delete);
            }

            // Find and remove the corresponding ToolResult that appears after the LlmResponse
            if let Some(tool_result_index) = self.inner.iter().skip(index).position(|i| {
                if let Interaction::ToolResult { tool_call_id, .. } = i {
                    tool_call_id == tool_call_id_to_delete
                } else {
                    false
                }
            }) {
                self.inner.remove(index + tool_result_index);
            }
        }

        // Finally, prune any LLM messages that have become empty
        self.inner.retain(|interaction| !interaction.is_empty());
    }

    pub fn delete_by_id(&mut self, id: usize) {
        let interaction_to_delete = self.inner.iter().find(|i| i.id() == id).cloned();

        if let Some(interaction) = interaction_to_delete {
            match interaction {
                Interaction::LlmResponse {
                    tool_calls: Some(calls),
                    interaction_id,
                    ..
                } => {
                    for call in calls {
                        self.delete_by_tool_id(interaction_id, &call.id);
                    }
                }
                Interaction::ToolResult { tool_call_id, .. } => {
                    // find the corrosponding llm response
                    if let Some(llm_interaction) = self.inner.iter().find(|i| {
                        if let Interaction::LlmResponse {
                            tool_calls: Some(calls),
                            ..
                        } = i
                        {
                            calls.iter().any(|c| c.id == tool_call_id)
                        } else {
                            false
                        }
                    }) {
                        self.delete_by_tool_id(llm_interaction.id(), &tool_call_id);
                    }
                }
                _ => {}
            }
        }
        self.inner.retain(|i| i.id() != id);
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
