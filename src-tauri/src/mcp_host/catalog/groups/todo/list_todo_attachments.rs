//! MCP API: `list_todo_attachments`

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
        todo_wire(todo_task::list_attachments(master_task_id), 200)
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "list_todo_attachments",
        "List attachments for a todo task.",
        object_schema(
            json!({ "master_task_id": { "type": "string", "description": "Todo task id." } }),
            &["master_task_id"],
        ),
        true,
        false,
        invoke,
    ))
}
