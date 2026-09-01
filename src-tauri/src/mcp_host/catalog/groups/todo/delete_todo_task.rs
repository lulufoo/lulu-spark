//! MCP API: `delete_todo_task`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{
    gated_todo, missing_field, object_schema, route, todo_wire,
};
use crate::mcp_host::ToolRoute;
use crate::services::todo_task;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    gated_todo(|| {
        let Some(master_task_id) = args.get("master_task_id").and_then(|v| v.as_str()) else {
            return missing_field("master_task_id");
        };
        todo_wire(todo_task::delete_master(master_task_id), 200)
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "delete_todo_task",
        "Delete a todo task and its contents.",
        object_schema(
            json!({ "master_task_id": { "type": "string", "description": "Todo task id." } }),
            &["master_task_id"],
        ),
        false,
        true,
        invoke,
    ))
}
