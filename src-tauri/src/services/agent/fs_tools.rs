//! Host file tools (grep / read / write / edit) plus Chat-scoped Stage trio.

use std::path::Path;

use serde_json::{json, Value};

use super::fs_file_ops as file_ops;
use super::mcp_client::{ToolCatalog, ToolResult};
use super::path_fence::PathFence;
use super::session::{self, StagedEntry};
use crate::services::id::random_hex12;

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
            "Register a file path on this Chat's Stage. path is required; title is optional and defaults to the last path segment without extension. Does not store file body.",
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
            "Return one Stage registration by id (id, path, title). Does not return file body; use read for contents.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Staged entry id." }
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

pub fn call(name: &str, arguments: &Value, fence: &PathFence) -> ToolResult {
    let result = match name {
        "grep" => file_ops::grep(arguments, fence),
        "read" => file_ops::read(arguments, fence),
        "write" => file_ops::write(arguments, fence),
        "edit" => file_ops::edit(arguments, fence),
        "stage" => stage(arguments),
        "list_staged" => list_staged(),
        "get_staged" => get_staged(arguments),
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

fn live_session_id() -> Result<String, String> {
    session::live_context_owner()
        .current_session_id()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| "no live session".into())
}

fn default_title(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn entry_json(entry: &StagedEntry) -> Result<String, String> {
    serde_json::to_string(entry).map_err(|e| e.to_string())
}

fn stage(arguments: &Value) -> Result<String, String> {
    let path = file_ops::arg_str(arguments, "path")?;
    if path.trim().is_empty() {
        return Err("missing path".into());
    }
    let title = arguments
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_title(&path));
    let sid = live_session_id()?;
    let mut sess = session::load_session(&sid)?;
    let entry = StagedEntry {
        id: format!("stg_{}", random_hex12()),
        path,
        title,
    };
    sess.staged.push(entry.clone());
    session::save_session(&sess)?;
    entry_json(&entry)
}

fn list_staged() -> Result<String, String> {
    let sess = session::load_session(&live_session_id()?)?;
    serde_json::to_string(&sess.staged).map_err(|e| e.to_string())
}

fn get_staged(arguments: &Value) -> Result<String, String> {
    let id = file_ops::arg_str(arguments, "id")?;
    let sess = session::load_session(&live_session_id()?)?;
    let entry = sess
        .staged
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "staged file not found".to_string())?;
    entry_json(entry)
}

#[cfg(test)]
#[path = "../../unit-tests/services/agent/fs_tools_tests.rs"]
mod tests;
