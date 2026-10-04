//! Chat-scoped Stage tools (stage / list_staged / get_staged).

use serde_json::{json, Value};

use crate::agent::session::{Session, StagedEntry};
use crate::services::path_fence::{validate_stage_file, PathFence};

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

pub const NOTE_CONTENT_TOOL: &str = "get_note_content";
pub const KNOWLEDGE_CONTENT_TOOL: &str = "get_knowledge_content";

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "stage",
            "Register an eligible regular file on this Chat's Stage and assign a staged document id (F1, F2, …). path must be an absolute readable file; title is optional. Same path reuses the existing id. Does not store file body.",
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

pub fn call(
    name: &str,
    arguments: &Value,
    session: &mut Session,
    fence: Option<&PathFence>,
) -> ToolResult {
    let result = match name {
        "stage" => stage(arguments, session, fence),
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

fn stage(
    arguments: &Value,
    session: &mut Session,
    fence: Option<&PathFence>,
) -> Result<String, String> {
    let Some(fence) = fence else {
        return Err("Host tool 'stage' has no path fence for this binding.".into());
    };
    let path = fs::arg_str(arguments, "path")?;
    let title = arguments.get("title").and_then(Value::as_str);
    let canon = validate_stage_file(&path, fence)?;
    entry_json(&session.register_staged(
        &canon.to_string_lossy(),
        title,
        None,
    )?)
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

/// After a spark path-tool success, register Stage and return F1/F2… only.
pub fn overlay_note_content(
    name: &str,
    result: ToolResult,
    session: &mut Session,
) -> (ToolResult, bool) {
    let (source_key, kind) = match name {
        NOTE_CONTENT_TOOL => ("note_id", "notes"),
        KNOWLEDGE_CONTENT_TOOL => ("knowledge_id", "knowledge"),
        _ => return (result, false),
    };
    if result.is_error {
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
                content: format!("{name} succeeded without a path"),
                is_error: true,
            },
            false,
        );
    };
    let source_id = value.get("id").and_then(Value::as_str).unwrap_or("");
    match session.register_staged(path, None, Some(kind)) {
        Ok(entry) => {
            let mut body = json!({
                "ok": true,
                "id": entry.id,
                "title": entry.title,
            });
            if let Some(obj) = body.as_object_mut() {
                obj.insert(source_key.to_string(), json!(source_id));
            }
            (
                ToolResult {
                    content: body.to_string(),
                    is_error: false,
                },
                true,
            )
        }
        Err(content) => (ToolResult { content, is_error: true }, false),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_stage_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/note_content_stage_tests.rs"]
mod overlay_tests;
