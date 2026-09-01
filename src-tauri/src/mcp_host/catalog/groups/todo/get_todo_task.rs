//! MCP API: `get_todo_task`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{
    arg_str, gated_todo, object_schema, route, todo_wire_read,
};
use crate::mcp_host::ToolRoute;
use crate::services::todo_task;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    gated_todo(|| todo_wire_read(todo_task::get_by_id(arg_str(args, "id").unwrap_or(""))))
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_todo_task",
        "Get one todo task by id.",
        object_schema(
            json!({ "id": { "type": "string", "description": "Todo task id." } }),
            &["id"],
        ),
        true,
        false,
        invoke,
    ))
}
