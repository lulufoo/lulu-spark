//! MCP API: `update_todo_sub`

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
        "update_todo_sub",
        "Update a subtask title and optional content. A title is always required.",
        HttpMethod::Post,
        "/api/todo-task-update-sub",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "sub_task_id": { "type": "string", "description": "Subtask id." },
                "title": { "type": "string", "description": "Replacement subtask title." },
                "content": { "type": "string", "description": "Optional replacement content." }
            }),
            &["master_task_id", "sub_task_id", "title"],
        ),
        false,
        false,
    ))
}
