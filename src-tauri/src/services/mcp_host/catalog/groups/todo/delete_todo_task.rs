//! MCP API: `delete_todo_task`

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
        "delete_todo_task",
        "Delete a todo task and its contents.",
        HttpMethod::Post,
        "/api/todo-task-delete",
        object_schema(
            json!({ "master_task_id": { "type": "string", "description": "Todo task id." } }),
            &["master_task_id"],
        ),
        false,
        true,
    ))
}
