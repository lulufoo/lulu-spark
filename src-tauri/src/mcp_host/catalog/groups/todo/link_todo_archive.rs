//! MCP API: `link_todo_archive`

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
        "link_todo_archive",
        "Link an archive entry to a todo subtask.",
        HttpMethod::Post,
        "/api/todo-task-link-archive",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "sub_task_id": { "type": "string", "description": "Subtask id." },
                "archive_id": { "type": "string", "description": "Archive entry id." }
            }),
            &["master_task_id", "sub_task_id", "archive_id"],
        ),
        false,
        false,
    ))
}
