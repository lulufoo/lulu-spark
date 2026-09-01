//! MCP API: `list_todo_tasks`

use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(_channel: &str) -> Option<ToolRoute> {
    if !available_in(_channel) {
        return None;
    }
    Some(route(
        "list_todo_tasks",
        "List todo tasks, optionally filtered by category id.",
        HttpMethod::Get,
        "/api/todo-tasks",
        object_schema(
            json!({
                "category_id": { "type": "string", "description": "Optional category id filter." }
            }),
            &[],
        ),
        true,
        false,
    ))
}
