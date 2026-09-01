//! MCP API: `complete_todo`

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
        "complete_todo",
        "Complete a todo task or one of its subtasks.",
        HttpMethod::Post,
        "/api/todo-task-complete",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "sub_task_id": { "type": "string", "description": "Optional subtask id." }
            }),
            &["master_task_id"],
        ),
        false,
        false,
    ))
}
