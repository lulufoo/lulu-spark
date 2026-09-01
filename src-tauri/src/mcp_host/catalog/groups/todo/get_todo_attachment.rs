//! MCP API: `get_todo_attachment`

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
        let Some(file_name) = args.get("file_name").and_then(|v| v.as_str()) else {
            return missing_field("file_name");
        };
        todo_wire(todo_task::read_attachment(master_task_id, file_name), 200)
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_todo_attachment",
        "Read one todo attachment by file name.",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "file_name": { "type": "string", "description": "Attachment file name." }
            }),
            &["master_task_id", "file_name"],
        ),
        true,
        false,
        invoke,
    ))
}
