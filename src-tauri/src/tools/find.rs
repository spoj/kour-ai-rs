use crate::Result;
use crate::error::Error;
use crate::search::search_files_by_name;

use super::{Function, Tool};
use serde::{Deserialize, Serialize};

pub fn get_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "find".to_string(),
            description: "Locates files by glob, returning up to 'list_max_files' matches. If more files match, it returns an error and the total count, prompting you to refine the glob. Excellent for targeted searches when you expect a manageable number of results. Use 'ls' to confirm existence or explore a directory before crafting a glob.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The pattern to match against. This is similar style to many desktop apps. the pattern is first shell-lexed into individual terms. each term is treated like a glob pattern. and terms are considered to be related by AND."
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum results. If glob matches more than this, the tool will return an error to avoid overwhelming the user. Start with 200 and adjust approach if required."
                    }
                },
                "required": ["pattern","max_results"]
            }),
        },
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FindArgs {
    pub pattern: String,
    pub max_results: usize,
}

pub async fn find(args: FindArgs) -> Result<Vec<String>> {
    let result = search_files_by_name(&args.pattern)?;

    if result.len() > args.max_results {
        return Err(Error::Tool(format!(
            "Error: Found more files ({}) than limit ({}). Consider up the limit or take different approach.",
            args.max_results,
            result.len()
        )));
    }
    Ok(result)
}
