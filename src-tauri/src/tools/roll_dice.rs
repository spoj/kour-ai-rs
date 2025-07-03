use crate::tools::{Function, Tool, ToolPayload};
use rand::Rng;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RollDiceArgs {}

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "roll_dice".to_string(),
            description: "Roll a 6-sided die".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    }
}

pub async fn execute(_args: RollDiceArgs) -> ToolPayload {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    ToolPayload::from(rand::rng().random_range(1..=6)).unwrap()
}
