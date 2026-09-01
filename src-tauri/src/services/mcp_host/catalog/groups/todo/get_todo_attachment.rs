//! MCP API: `get_todo_attachment`

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
        "get_todo_attachment",
        "Read one todo attachment by file name.",
        HttpMethod::Post,
        "/api/todo-task-get-attachment",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "file_name": { "type": "string", "description": "Attachment file name." }
            }),
            &["master_task_id", "file_name"],
        ),
        true,
        false,
    ))
}
