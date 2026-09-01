//! MCP API: `delete_todo_sub`

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
        let Some(sub_task_id) = args.get("sub_task_id").and_then(|v| v.as_str()) else {
            return missing_field("sub_task_id");
        };
        todo_wire(todo_task::delete_sub(master_task_id, sub_task_id), 200)
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "delete_todo_sub",
        "Delete a subtask.",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "sub_task_id": { "type": "string", "description": "Subtask id." }
            }),
            &["master_task_id", "sub_task_id"],
        ),
        false,
        true,
        invoke,
    ))
}
