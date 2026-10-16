//! Chat-scoped Stage tools (stage / list_staged / get_staged).

use serde_json::{json, Value};

use crate::agent::session::{Session, StagedEntry};
use crate::agent::fence::{validate_stage_file, PathFence};

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "stage",
            "Add an external file to this Chat's Stage. Asks the user first. After allow, grants read and write for that path and shows it to the user. The path may not exist yet. Spark's own data and .git cannot be requested. Workspace documents are not staged: use get_doc_file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path. May not exist yet. Must be outside SESSION_WORKSPACE_DIR." },
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
    into_tool_result(result)
}

fn into_tool_result(result: Result<String, String>) -> ToolResult {
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

/// A `stage` request whose file passed the disk checks. Registering it only touches memory.
pub struct CheckedStage {
    path: String,
    title: Option<String>,
}

impl CheckedStage {
    pub(crate) fn path_for_gate(&self) -> &str {
        &self.path
    }
}

/// The part of `stage` that reads the disk. It owns everything it returns, so it can run
/// on another thread.
pub fn check_stage(arguments: &Value, fence: Option<&PathFence>) -> Result<CheckedStage, String> {
    let Some(fence) = fence else {
        return Err("Host tool 'stage' has no path fence for this binding.".into());
    };
    let path = fs::arg_str(arguments, "path")?;
    let title = arguments
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string);
    let canon = validate_stage_file(&path, fence)?;
    Ok(CheckedStage {
        path: canon.to_string_lossy().into_owned(),
        title,
    })
}

/// The part of `stage` that changes the Session. Pure memory, so it never needs a time limit.
pub fn register_stage(session: &mut Session, checked: CheckedStage) -> ToolResult {
    into_tool_result(register(session, checked))
}

fn register(session: &mut Session, checked: CheckedStage) -> Result<String, String> {
    let entry = session.register_staged(&checked.path, checked.title.as_deref())?;
    entry_json(&entry)
}

fn stage(
    arguments: &Value,
    session: &mut Session,
    fence: Option<&PathFence>,
) -> Result<String, String> {
    register(session, check_stage(arguments, fence)?)
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
