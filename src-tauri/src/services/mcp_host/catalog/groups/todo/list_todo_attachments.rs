//! MCP API: `list_todo_attachments`

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
        "list_todo_attachments",
        "List attachments for a todo task.",
        HttpMethod::Post,
        "/api/todo-task-list-attachments",
        object_schema(
            json!({ "master_task_id": { "type": "string", "description": "Todo task id." } }),
            &["master_task_id"],
        ),
        true,
        false,
    ))
}
