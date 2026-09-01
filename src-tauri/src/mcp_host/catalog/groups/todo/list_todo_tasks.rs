//! MCP API: `list_todo_tasks`

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
    gated_todo(|| {
        let value = todo_wire_read(todo_task::list_all());
        if !value.is_array() {
            return value;
        }
        let Some(category_id) = arg_str(args, "category_id").map(str::trim).filter(|s| !s.is_empty())
        else {
            return value;
        };
        Value::Array(
            value
                .as_array()
                .into_iter()
                .flatten()
                .filter(|t| t.get("category_id").and_then(|v| v.as_str()) == Some(category_id))
                .cloned()
                .collect(),
        )
    })
}

pub fn build(_channel: &str) -> Option<ToolRoute> {
    if !available_in(_channel) {
        return None;
    }
    Some(route(
        "list_todo_tasks",
        "List todo tasks, optionally filtered by category id.",
        object_schema(
            json!({
                "category_id": { "type": "string", "description": "Optional category id filter." }
            }),
            &[],
        ),
        true,
        false,
        invoke,
    ))
}
