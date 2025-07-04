use crate::{
    Result,
    interaction::{Interaction, Source, Target},
    openrouter::{ChatOptions, Openrouter, ToolCall},
    tools::{self, ToolPayload},
    ui_events::UIEvents,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::to_string;

pub static SYSTEM_PROMPT: &str = include_str!("DEFAULT_PROMPT.md");

pub struct ChatProcessor {
    ui: UIEvents,
    options: ChatOptions,
    interactions: Vec<Interaction>,
}

impl ChatProcessor {
    pub fn new(window: tauri::Window, options: ChatOptions, messages: Vec<Interaction>) -> Self {
        Self {
            ui: UIEvents::new(window),
            options,
            interactions: messages,
        }
    }

    pub async fn run(mut self) -> Result<Vec<Interaction>> {
        let _ = self.ui.emit_start();

        loop {
            let to_llm: Vec<_> = Openrouter::render(&self.interactions);
            let res = Openrouter::call(
                &to_llm,
                &self.options.model_name,
                SYSTEM_PROMPT,
                &tools::TOOLS,
                None,
            )
            .await?;

            let choice = &res.choices[0];
            let incoming_message = choice.message.clone();
            let interaction = Openrouter::sends(incoming_message.clone());

            if let Some(tool_calls) = incoming_message.tool_calls.clone() {
                self.interactions.push(interaction);

                let new_interactions = self.handle_tool_calls(tool_calls).await?;
                for msg in new_interactions {
                    let tool_message = msg;
                    self.interactions.push(tool_message);
                }
                // After handling tools, continue the loop to let the assistant respond.
            } else {
                // It's a final text response. Add it to history, emit, and break the loop.
                self.interactions.push(interaction.clone());
                self.ui.emit_interaction(&interaction)?;
                break;
            }
        }

        self.ui.emit_done()?;

        Ok(self.interactions)
    }

    async fn execute_tool_call(replayer: UIEvents, tool_call: ToolCall) -> Interaction {
        let _ = replayer.emit_tool_call(
            &tool_call.function.name,
            &tool_call.id,
            &tool_call.function.arguments,
        );

        let tool_payload =
            tools::tool_executor(&tool_call.function.name, &tool_call.function.arguments).await;

        let result = to_string(&tool_payload.response).unwrap_or("Json error".to_string());
        let _ = replayer.emit_tool_result(&tool_call.id, &result);

        tool_payload.finalize(tool_call.id.to_string()).unwrap()
    }

    pub async fn handle_tool_calls(&self, tool_calls: Vec<ToolCall>) -> Result<Vec<Interaction>> {
        let mut new_messages = Vec::new();

        let tool_futs = tool_calls
            .into_iter()
            .map(|tool_call| tokio::spawn(Self::execute_tool_call(self.ui.clone(), tool_call)));

        let tool_payloads = join_all(tool_futs).await;

        for tool_payload in tool_payloads {
            new_messages.push(tool_payload?);
        }

        Ok(new_messages)
    }
}
