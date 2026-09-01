//! MCP API: `delete_todo_sub`

use serde_json::json;

use crate::services::mcp_host::catalog::route_util::{object_schema, route};
use crate::services::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "delete_todo_sub",
        "Delete a subtask.",
        HttpMethod::Post,
        "/api/todo-task-delete-sub",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "sub_task_id": { "type": "string", "description": "Subtask id." }
            }),
            &["master_task_id", "sub_task_id"],
        ),
        false,
        true,
    ))
}
