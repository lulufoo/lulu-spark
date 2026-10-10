//! Host file tools (grep / read / write / str_replace / copy) under PathFence.

use serde_json::{json, Value};

use crate::services::path_fence::PathFence;

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "grep",
            "Search text.\n\nReadable paths: SESSION_WORKSPACE_DIR and files on this Chat's Stage.\npath is optional; omit path to search the workspace and every staged file.",
            json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regex to search for." },
                    "path": { "type": "string", "description": "Optional absolute file or directory." }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            true,
        ),
        local_tool(
            "read",
            "Read a text file.\nOptional offset (1-based start line) and limit (max lines, default 50).\n\nReadable paths: SESSION_WORKSPACE_DIR and files on this Chat's Stage.\npath must be absolute.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path." },
                    "offset": { "type": "integer", "description": "1-based start line." },
                    "limit": { "type": "integer", "description": "Max lines to return. Defaults to 50." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            true,
        ),
        local_tool(
            "write",
            "Create or overwrite a text file in SESSION_WORKSPACE_DIR or on this Chat's Stage.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path. Must be in SESSION_WORKSPACE_DIR or an existing file on Stage." },
                    "content": { "type": "string", "description": "Full file contents." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            false,
        ),
        local_tool(
            "str_replace",
            "Replace exact text in an existing text file in SESSION_WORKSPACE_DIR or on this Chat's Stage.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path. Must be in SESSION_WORKSPACE_DIR or an existing file on Stage." },
                    "old_string": { "type": "string", "description": "Text to replace. Must match exactly once unless replace_all is true." },
                    "new_string": { "type": "string", "description": "The text to replace it with." },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences of old_string (default false)."
                    }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
            false,
        ),
        local_tool(
            "copy",
            "Copy a regular file. Does not modify the source. Overwrites dest_path if it already exists.",
            json!({
                "type": "object",
                "properties": {
                    "source_path": { "type": "string", "description": "Absolute readable file in SESSION_WORKSPACE_DIR or on Stage." },
                    "dest_path": { "type": "string", "description": "Absolute file path in SESSION_WORKSPACE_DIR or an existing file on Stage. Not a directory." }
                },
                "required": ["source_path", "dest_path"],
                "additionalProperties": false
            }),
            false,
        ),
    ])
}

pub fn is_builtin(name: &str) -> bool {
    matches!(name, "grep" | "read" | "write" | "str_replace" | "copy")
}

pub fn call(name: &str, arguments: &Value, fence: &PathFence) -> ToolResult {
    let result = match name {
        "grep" => fs::grep(arguments, fence),
        "read" => fs::read(arguments, fence),
        "write" => fs::write(arguments, fence),
        "str_replace" => fs::str_replace(arguments, fence),
        "copy" => fs::copy(arguments, fence),
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

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_host_tests.rs"]
mod tests;
