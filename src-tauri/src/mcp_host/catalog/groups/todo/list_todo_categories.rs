//! MCP API: `list_todo_categories`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{gated_todo, object_schema, route, todo_wire};
use crate::mcp_host::ToolRoute;
use crate::services::todo_task;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(_args: &Value) -> Value {
    gated_todo(|| todo_wire(todo_task::list_todo_categories(), 200))
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "list_todo_categories",
        "List all todo categories.",
        object_schema(json!({}), &[]),
        true,
        false,
        invoke,
    ))
}
