//! MCP API: `update_todo_task`

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
        "update_todo_task",
        "Update a todo task's title, Markdown body, and/or category. Provide at least one field to change.",
        HttpMethod::Post,
        "/api/todo-task-update",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "title": { "type": "string", "description": "Replacement task title." },
                "todo_md": { "type": "string", "description": "Replacement task body; empty clears it." },
                "category_id": { "type": "string", "description": "Replacement category id." }
            }),
            &["master_task_id"],
        ),
        false,
        false,
    ))
}
