//! MCP API: `create_todo_task`

use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "create_todo_task",
        "Create a todo task with a title and required Markdown body, plus optional category and initial subtask titles.",
        HttpMethod::Post,
        "/api/todo-task-create",
        object_schema(
            json!({
                "title": { "type": "string", "description": "Todo task title." },
                "todo_md": { "type": "string", "description": "Required task body Markdown. Blank is rejected." },
                "category_id": { "type": "string", "description": "Optional todo category id." },
                "sub_titles": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional initial subtask titles."
                }
            }),
            &["title", "todo_md"],
        ),
        false,
        false,
    ))
}
