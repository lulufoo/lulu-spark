//! MCP API: `update_todo_task`

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
        let title = match args.get("title") {
            None => None,
            Some(Value::String(s)) => Some(s.as_str()),
            Some(_) => return json!({ "error": "Invalid title", "_status": 400 }),
        };
        let todo_md = match args.get("todo_md") {
            None => None,
            Some(Value::String(s)) => Some(s.as_str()),
            Some(_) => return json!({ "error": "Invalid todo_md", "_status": 400 }),
        };
        let category_id = match args.get("category_id") {
            None => None,
            Some(Value::String(s)) => Some(s.as_str()),
            Some(_) => return json!({ "error": "Invalid category_id", "_status": 400 }),
        };
        if title.is_none() && todo_md.is_none() && category_id.is_none() {
            return json!({ "error": "Missing title or todo_md", "_status": 400 });
        }
        if title.is_none() && todo_md.is_none() {
            return todo_wire(
                todo_task::set_master_category(master_task_id, category_id.unwrap_or("")),
                200,
            );
        }
        let updated = todo_wire(
            todo_task::update_master_fields(master_task_id, title, todo_md),
            200,
        );
        if updated.get("_status").and_then(|v| v.as_u64()).unwrap_or(200) >= 400 {
            return updated;
        }
        match category_id {
            Some(cid) => todo_wire(todo_task::set_master_category(master_task_id, cid), 200),
            None => updated,
        }
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "update_todo_task",
        "Update a todo task's title, Markdown body, and/or category. Provide at least one field to change.",
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
        invoke,
    ))
}
