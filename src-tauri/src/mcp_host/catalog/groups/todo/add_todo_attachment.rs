//! MCP API: `add_todo_attachment`

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
        if args.get("content").is_some() {
            return json!({
                "error": "content is not supported; use source_path",
                "_status": 400
            });
        }
        let Some(master_task_id) = args.get("master_task_id").and_then(|v| v.as_str()) else {
            return missing_field("master_task_id");
        };
        let Some(source_path) = args.get("source_path").and_then(|v| v.as_str()) else {
            return missing_field("source_path");
        };
        todo_wire(todo_task::add_attachment(master_task_id, source_path), 201)
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "add_todo_attachment",
        "Copy a local Markdown file into a todo task as an attachment. Use source_path, never file content.",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "source_path": { "type": "string", "description": "Absolute source Markdown path." }
            }),
            &["master_task_id", "source_path"],
        ),
        false,
        false,
        invoke,
    ))
}
