//! MCP API: `update_todo_attachment`

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
        let Some(file_name) = args.get("file_name").and_then(|v| v.as_str()) else {
            return missing_field("file_name");
        };
        let Some(source_path) = args.get("source_path").and_then(|v| v.as_str()) else {
            return missing_field("source_path");
        };
        todo_wire(
            todo_task::save_attachment(master_task_id, file_name, source_path),
            200,
        )
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "update_todo_attachment",
        "Replace an existing todo attachment by copying a local Markdown file. Use source_path, never file content.",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "file_name": { "type": "string", "description": "Existing attachment file name." },
                "source_path": { "type": "string", "description": "Absolute replacement Markdown path." }
            }),
            &["master_task_id", "file_name", "source_path"],
        ),
        false,
        true,
        invoke,
    ))
}
