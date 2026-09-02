//! MCP API: `create_todo_task`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{
    gated_todo, missing_field, object_schema, route, todo_wire,
};
use crate::mcp_host::ToolRoute;
use crate::services::todo_task;

pub fn available_in(_channel: &str) -> bool {
    true
}

fn parse_todo_md(args: &Value) -> Result<&str, Value> {
    match args.get("todo_md") {
        None | Some(Value::Null) => Err(missing_field("todo_md")),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                Err(missing_field("todo_md"))
            } else {
                Ok(s.as_str())
            }
        }
        Some(_) => Err(json!({ "error": "Invalid todo_md", "_status": 400 })),
    }
}

fn parse_category_id(args: &Value) -> Result<Option<&str>, Value> {
    match args.get("category_id") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(json!({ "error": "Invalid category_id", "_status": 400 })),
    }
}

pub fn invoke(args: &Value) -> Value {
    gated_todo(|| {
        let Some(title) = args.get("title").and_then(|v| v.as_str()) else {
            return missing_field("title");
        };
        let todo_md = match parse_todo_md(args) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let category_id = match parse_category_id(args) {
            Ok(v) => v,
            Err(err) => return err,
        };
        todo_wire(
            todo_task::create_master_with_category(title, None, todo_md, category_id),
            201,
        )
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "create_todo_task",
        "Create a todo task with a title and required Markdown body. Does not create subtasks; add them with add_todo_sub.",
        object_schema(
            json!({
                "title": { "type": "string", "description": "Todo task title." },
                "todo_md": { "type": "string", "description": "Required task body Markdown. Blank is rejected." },
                "category_id": { "type": "string", "description": "Optional todo category id." }
            }),
            &["title", "todo_md"],
        ),
        false,
        false,
        invoke,
    ))
}
