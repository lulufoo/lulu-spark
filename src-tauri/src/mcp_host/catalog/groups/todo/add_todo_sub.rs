//! MCP API: `add_todo_sub`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{
    gated_todo, missing_field, object_schema, require_nonempty_str, route, todo_wire,
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
        let Some(title) = args.get("title").and_then(|v| v.as_str()) else {
            return missing_field("title");
        };
        let content = match require_nonempty_str(args, "content") {
            Ok(v) => v,
            Err(err) => return err,
        };
        todo_wire(
            todo_task::add_sub(master_task_id, title, Some(content)),
            201,
        )
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "add_todo_sub",
        "Add a subtask with a title and required content.",
        object_schema(
            json!({
                "master_task_id": { "type": "string", "description": "Todo task id." },
                "title": { "type": "string", "description": "Subtask title." },
                "content": { "type": "string", "description": "Required subtask content." }
            }),
            &["master_task_id", "title", "content"],
        ),
        false,
        false,
        invoke,
    ))
}
