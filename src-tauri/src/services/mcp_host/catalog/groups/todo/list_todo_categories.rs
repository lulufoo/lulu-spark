//! MCP API: `list_todo_categories`

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
        "list_todo_categories",
        "List all todo categories.",
        HttpMethod::Get,
        "/api/todo-task-list-categories",
        object_schema(json!({}), &[]),
        true,
        false,
    ))
}
