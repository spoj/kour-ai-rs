use crate::tools::{Function, Tool};
use rand::Rng;

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

pub async fn execute(_arguments: &str) -> String {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let roll = rand::thread_rng().gen_range(1..=6);
    roll.to_string()
}
