//! Host file tools (grep / read / write / edit) plus Chat-scoped Stage trio.

use std::path::Path;

use serde_json::{json, Value};

use super::fs_file_ops as file_ops;
use super::mcp_client::{ToolCatalog, ToolResult};
use super::path_fence::PathFence;
use super::session::{Session, StagedEntry};

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "grep",
            "Search file contents under the read-allowed roots. path is optional; omit to search all read roots. Pattern is a Rust regex.",
            json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regex to search for." },
                    "path": { "type": "string", "description": "Optional absolute path under a read root." }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            true,
        ),
        local_tool(
            "read",
            "Read a text file under a read-allowed root. path must be absolute. Optional 1-based offset and limit (default 2000 lines).",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path." },
                    "offset": { "type": "integer", "description": "1-based start line." },
                    "limit": { "type": "integer", "description": "Max lines to return." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            true,
        ),
        local_tool(
            "write",
            "Create or overwrite a text file. path must be absolute and under the write-allowed scratch root.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path." },
                    "content": { "type": "string", "description": "Full file contents." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            false,
        ),
        local_tool(
            "edit",
            "Replace exact text in an existing file under the write-allowed scratch root. old_text must match exactly once.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path." },
                    "old_text": { "type": "string", "description": "Exact text to find." },
                    "new_text": { "type": "string", "description": "Replacement text." }
                },
                "required": ["path", "old_text", "new_text"],
                "additionalProperties": false
            }),
            false,
        ),
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
    matches!(
        name,
        "grep" | "read" | "write" | "edit" | "stage" | "list_staged" | "get_staged"
    )
}

/// Host tools. Stage trio mutates `session` in memory (turn persists); file tools ignore it.
pub fn call(
    name: &str,
    arguments: &Value,
    fence: &PathFence,
    session: &mut Session,
) -> ToolResult {
    let result = match name {
        "grep" => file_ops::grep(arguments, fence),
        "read" => file_ops::read(arguments, fence),
        "write" => file_ops::write(arguments, fence),
        "edit" => file_ops::edit(arguments, fence),
        "stage" => stage(arguments, session),
        "list_staged" => list_staged(session),
        "get_staged" => get_staged(arguments, session),
        other => Err(format!("unknown file tool '{other}'")),
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

pub(crate) struct LocalTool {
    name: &'static str,
    description: &'static str,
    parameters: Value,
    read_only: bool,
}

impl ToolCatalog {
    pub(crate) fn from_local_tools(tools: Vec<LocalTool>) -> Self {
        let mut definitions = Vec::new();
        let mut names = std::collections::BTreeSet::new();
        let mut read_only_names = std::collections::BTreeSet::new();
        let mut input_schemas = std::collections::BTreeMap::new();
        for tool in tools {
            names.insert(tool.name.to_string());
            if tool.read_only {
                read_only_names.insert(tool.name.to_string());
            }
            input_schemas.insert(tool.name.to_string(), tool.parameters.clone());
            definitions.push(json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            }));
        }
        Self::from_parts(definitions, names, read_only_names, input_schemas)
    }
}

fn default_title(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn parse_handle(id: &str) -> Option<u32> {
    let rest = id.strip_prefix('F')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn next_handle(staged: &[StagedEntry]) -> String {
    let high = staged.iter().filter_map(|e| parse_handle(&e.id)).max().unwrap_or(0);
    format!("F{}", high + 1)
}

fn entry_json(entry: &StagedEntry) -> Result<String, String> {
    serde_json::to_string(entry).map_err(|e| e.to_string())
}

/// Register a path on the turn session in memory. Caller persists (`persist` / `save_session`).
pub(crate) fn stage_into(
    session: &mut Session,
    path: &str,
    title: Option<&str>,
) -> Result<StagedEntry, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("missing path".to_string());
    }
    let title = title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_title(path));
    let entry = StagedEntry {
        id: next_handle(&session.staged),
        path: path.to_string(),
        title,
    };
    session.staged.push(entry.clone());
    Ok(entry)
}

fn stage(arguments: &Value, session: &mut Session) -> Result<String, String> {
    let path = file_ops::arg_str(arguments, "path")?;
    let title = arguments.get("title").and_then(Value::as_str);
    entry_json(&stage_into(session, &path, title)?)
}

fn list_staged(session: &Session) -> Result<String, String> {
    serde_json::to_string(&session.staged).map_err(|e| e.to_string())
}

fn get_staged(arguments: &Value, session: &Session) -> Result<String, String> {
    let id = file_ops::arg_str(arguments, "id")?;
    let entry = session
        .staged
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "staged file not found".to_string())?;
    entry_json(entry)
}

#[cfg(test)]
#[path = "../../unit-tests/services/agent/fs_tools_tests.rs"]
mod tests;
