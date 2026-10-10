//! Chat-scoped Stage tools (stage / list_staged / get_staged).

use serde_json::{json, Value};

use crate::agent::session::{Session, StagedEntry};
use crate::services::path_fence::{validate_stage_file, PathFence};

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "stage",
            "Add an external file to this Chat's Stage. Grants read and write for that file and shows it to the user. Use it when the user gives an absolute path to a file outside Spark's own data. Notes and knowledge documents are not staged: use get_note_file or get_knowledge_file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute existing regular file. Must be outside SESSION_WORKSPACE_DIR." },
                    "title": { "type": "string", "description": "Optional display title." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            false,
        ),
        local_tool(
            "list_staged",
            "List this Chat's Stage items (id, path, title, optional source). Does not return file bodies.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        local_tool(
            "get_staged",
            "Return one Stage item by staged document id (F1, F2, …). Returns id, path, title, and optional source. Does not return file body.",
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
    entry_json(&session.register_staged(&canon.to_string_lossy(), title, None)?)
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

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_stage_tests.rs"]
mod tests;
