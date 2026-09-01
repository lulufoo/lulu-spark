//! MCP API: `add_todo_attachment`

use serde_json::json;

use crate::services::mcp_host::catalog::route_util::{object_schema, route};
use crate::services::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "add_todo_attachment",
        "Copy a local Markdown file into a todo task as an attachment. Use source_path, never file content.",
        HttpMethod::Post,
        "/api/todo-task-add-attachment",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "source_path": { "type": "string", "description": "Absolute source Markdown path." }
            }),
            &["master_task_id", "source_path"],
        ),
        false,
        false,
    ))
}
