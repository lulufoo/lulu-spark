//! MCP API: `get_todo_task`

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
        "get_todo_task",
        "Get one todo task by id.",
        HttpMethod::Get,
        "/api/todo-task",
        object_schema(
            json!({ "id": { "type": "string", "description": "Todo task id." } }),
            &["id"],
        ),
        true,
        false,
    ))
}
