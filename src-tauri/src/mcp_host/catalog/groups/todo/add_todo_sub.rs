//! MCP API: `add_todo_sub`

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
        "add_todo_sub",
        "Add a subtask to a todo task.",
        HttpMethod::Post,
        "/api/todo-task-add-sub",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "title": { "type": "string", "description": "Subtask title." },
                "content": { "type": "string", "description": "Optional subtask content." }
            }),
            &["master_task_id", "title"],
        ),
        false,
        false,
    ))
}
