use std::sync::{Arc, Mutex};

use crate::{
    Result,
    interaction::{History, Source, Target},
    openrouter::{ChatOptions, Openrouter, ToolCall},
    tools,
    ui_events::UIEvents,
};
use futures::future::join_all;

pub static SYSTEM_PROMPT: &str = include_str!("DEFAULT_PROMPT.md");

pub struct ChatProcessor {
    ui: UIEvents,
    options: ChatOptions,
    history: Arc<Mutex<History>>,
}

impl ChatProcessor {
    pub fn new(window: tauri::Window, options: ChatOptions, history: Arc<Mutex<History>>) -> Self {
        Self {
            ui: UIEvents::new(window),
            options,
            history,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let _ = self.ui.emit_start();

        loop {
            let to_llm: Vec<_> = Openrouter::render(&self.history.lock().unwrap());
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
            self.ui.emit_interaction(&interaction)?;
            self.history.lock().unwrap().push(interaction);

            if let Some(tool_calls) = incoming_message.tool_calls.clone() {
                self.handle_tool_calls(tool_calls).await?;
            } else {
                break;
            }
        }

        self.ui.emit_done()?;

        Ok(())
    }

    pub async fn handle_tool_calls(&self, tool_calls: Vec<ToolCall>) -> Result<()> {
        let tool_futs = tool_calls.into_iter().map(async |tool_call| {
            let tool_payload =
                tools::tool_dispatcher(&tool_call.function.name, &tool_call.function.arguments)
                    .await;

            let interaction = tool_payload.finalize(tool_call.id.to_string());
            let _ = self.ui.emit_interaction(&interaction);
            self.history.lock().unwrap().push(interaction);
        });
        let _ = join_all(tool_futs).await;
        Ok(())
    }
}
