//! Shared MCP tool route constructors and in-process invoke helpers.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::mcp_host::{ToolInvoke, ToolRoute};
use crate::services::todo_task::{self, TodoError};

pub fn object_schema(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

pub fn route(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
    invoke: ToolInvoke,
) -> ToolRoute {
    ToolRoute {
        name: name.into(),
        description: description.into(),
        read_only,
        destructive,
        input_schema,
        invoke,
    }
}

pub fn notes_repo_root() -> Result<PathBuf, Value> {
    crate::config::paths::repo_root()
        .map_err(|_| json!({ "error": "Repo root unavailable", "_status": 500 }))
}

pub fn missing_field(name: &str) -> Value {
    json!({ "error": format!("Missing {name}"), "_status": 400 })
}

pub fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_field(key))
}

pub fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

pub fn require_nonempty_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Value> {
    match args.get(key) {
        None | Some(Value::Null) => Err(missing_field(key)),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Err(missing_field(key))
            } else {
                Ok(trimmed)
            }
        }
        Some(_) => Err(json!({ "error": format!("Invalid {key}"), "_status": 400 })),
    }
}

pub fn optional_nonempty_str<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, Value> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Err(missing_field(key))
            } else {
                Ok(Some(trimmed))
            }
        }
        Some(_) => Err(json!({ "error": format!("Invalid {key}"), "_status": 400 })),
    }
}

pub fn todo_wire(result: Result<Value, TodoError>, ok_status: u16) -> Value {
    todo_task::into_wire(result, ok_status)
}

pub fn todo_wire_read(result: Result<Value, TodoError>) -> Value {
    todo_task::into_wire_read(result)
}

pub fn gated_todo(run: impl FnOnce() -> Value) -> Value {
    match todo_task::ensure_todo_api_ungated() {
        Ok(()) => run(),
        Err(err) => err.into_wire(),
    }
}
