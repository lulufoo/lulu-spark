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

fn parse_sub_titles(args: &Value) -> Result<Option<Vec<String>>, Value> {
    match args.get("sub_titles") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(arr)) if arr.is_empty() => Ok(None),
        Some(Value::Array(arr)) => {
            let mut subs = Vec::with_capacity(arr.len());
            for item in arr {
                let Some(raw) = item.as_str() else {
                    return Err(json!({ "error": "Invalid sub_titles element", "_status": 400 }));
                };
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err(json!({ "error": "Invalid sub_titles element", "_status": 400 }));
                }
                subs.push(trimmed.to_string());
            }
            Ok(Some(subs))
        }
        Some(_) => Err(json!({ "error": "Invalid sub_titles", "_status": 400 })),
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
        let sub_titles = match parse_sub_titles(args) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match &sub_titles {
            Some(subs) => {
                let refs: Vec<&str> = subs.iter().map(String::as_str).collect();
                todo_wire(
                    todo_task::create_master_with_category(title, Some(&refs), todo_md, category_id),
                    201,
                )
            }
            None => todo_wire(
                todo_task::create_master_with_category(title, None, todo_md, category_id),
                201,
            ),
        }
    })
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "create_todo_task",
        "Create a todo task with a title and required Markdown body, plus optional category and initial subtask titles.",
        object_schema(
            json!({
                "title": { "type": "string", "description": "Todo task title." },
                "todo_md": { "type": "string", "description": "Required task body Markdown. Blank is rejected." },
                "category_id": { "type": "string", "description": "Optional todo category id." },
                "sub_titles": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional initial subtask titles."
                }
            }),
            &["title", "todo_md"],
        ),
        false,
        false,
        invoke,
    ))
}
