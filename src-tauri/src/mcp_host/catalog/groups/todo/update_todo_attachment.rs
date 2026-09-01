//! MCP API: `update_todo_attachment`

use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "update_todo_attachment",
        "Replace an existing todo attachment by copying a local Markdown file. Use source_path, never file content.",
        HttpMethod::Post,
        "/api/todo-task-update-attachment",
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
    ))
}
