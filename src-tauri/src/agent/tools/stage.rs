//! Chat-scoped Stage tools (stage / list_staged / get_staged).
//!
//! Depends on session Stage CRUD only — no path fence, no UI semantics.

use serde_json::{json, Value};

use crate::agent::session::{Session, StagedEntry};

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

pub const NOTE_CONTENT_TOOL: &str = "get_note_content_by_id";

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "stage",
            "Register a file path on this Chat's Stage and assign a staged document id (F1, F2, …). path is required; title is optional and defaults to the last path segment without extension. Does not store file body.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to register." },
                    "title": { "type": "string", "description": "Optional display title." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            false,
        ),
        local_tool(
            "list_staged",
            "List this Chat's Stage registrations (id, path, title). Does not return file bodies.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        local_tool(
            "get_staged",
            "Return one Stage registration by staged document id (F1, F2, …). Returns id, path, and title. Does not return file body.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Staged document id (F1, F2, …)." }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
            true,
        ),
    ])
}

pub fn is_builtin(name: &str) -> bool {
    matches!(name, "stage" | "list_staged" | "get_staged")
}

pub fn call(name: &str, arguments: &Value, session: &mut Session) -> ToolResult {
    let result = match name {
        "stage" => stage(arguments, session),
        "list_staged" => list_staged(session),
        "get_staged" => get_staged(arguments, session),
        other => Err(format!("unknown stage tool '{other}'")),
    };
    match result {
        Ok(content) => ToolResult {
            content,
            is_error: false,
        },
        Err(content) => ToolResult {
            content,
            is_error: true,
        },
    }
}

fn local_tool(
    name: &'static str,
    description: &'static str,
    parameters: Value,
    read_only: bool,
) -> LocalTool {
    LocalTool {
        name,
        description,
        parameters,
        read_only,
    }
}

fn entry_json(entry: &StagedEntry) -> Result<String, String> {
    serde_json::to_string(entry).map_err(|e| e.to_string())
}

fn stage(arguments: &Value, session: &mut Session) -> Result<String, String> {
    let path = fs::arg_str(arguments, "path")?;
    let title = arguments.get("title").and_then(Value::as_str);
    entry_json(&session.register_staged(&path, title)?)
}

fn list_staged(session: &Session) -> Result<String, String> {
    serde_json::to_string(session.list_staged()).map_err(|e| e.to_string())
}

fn get_staged(arguments: &Value, session: &Session) -> Result<String, String> {
    let id = fs::arg_str(arguments, "id")?;
    let entry = session
        .get_staged(&id)
        .ok_or_else(|| "staged file not found".to_string())?;
    entry_json(entry)
}

/// After a workbench note-path success, register Stage and return F1/F2… only.
pub fn overlay_note_content(
    name: &str,
    result: ToolResult,
    session: &mut Session,
) -> (ToolResult, bool) {
    if name != NOTE_CONTENT_TOOL || result.is_error {
        return (result, false);
    }
    let Ok(value) = serde_json::from_str::<Value>(&result.content) else {
        return (result, false);
    };
    if value.get("ok") != Some(&Value::Bool(true)) {
        return (result, false);
    }
    let Some(path) = value
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            ToolResult {
                content: "get_note_content_by_id succeeded without a path".into(),
                is_error: true,
            },
            false,
        );
    };
    let note_id = value.get("id").and_then(Value::as_str).unwrap_or("");
    match session.register_staged(path, None) {
        Ok(entry) => (
            ToolResult {
                content: json!({
                    "ok": true,
                    "id": entry.id,
                    "title": entry.title,
                    "note_id": note_id,
                })
                .to_string(),
                is_error: false,
            },
            true,
        ),
        Err(content) => (ToolResult { content, is_error: true }, false),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_stage_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/note_content_stage_tests.rs"]
mod overlay_tests;
