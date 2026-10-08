//! Host file tools (grep / read / write / str_replace / copy) under PathFence.

use serde_json::{json, Value};

use crate::services::path_fence::PathFence;

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

pub fn catalog() -> ToolCatalog {
    ToolCatalog::from_local_tools(vec![
        local_tool(
            "grep",
            "Search text.\n\nReadable paths: SPARK_DATA_DIR, SESSION_SCRATCH_DIR.\npath is optional; omit path to search both directories.",
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
            "Read a text file.\nOptional offset (1-based start line) and limit (max lines, default 50).\n\nReadable paths: SPARK_DATA_DIR, SESSION_SCRATCH_DIR.\npath must be absolute.",
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
            "Create or overwrite a text file.\n\nWritable paths: SESSION_SCRATCH_DIR.\nDo not write files in SPARK_DATA_DIR with this tool. Copy them to SESSION_SCRATCH_DIR to edit; use an MCP tool to update SPARK_DATA_DIR files.",
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
            "str_replace",
            "Replace exact text in an existing text file.\nold_string must match exactly once unless replace_all is true.\n\nWritable paths: SESSION_SCRATCH_DIR.\nDo not modify files in SPARK_DATA_DIR with this tool. Copy them to SESSION_SCRATCH_DIR to edit; use an MCP tool to update SPARK_DATA_DIR files.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path." },
                    "old_string": { "type": "string", "description": "The text to replace." },
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
            "Copy a regular file.\nDoes not modify the source. Overwrites dest_path if it already exists.\n\nSource paths: SPARK_DATA_DIR, SESSION_SCRATCH_DIR.\nDestination paths: SESSION_SCRATCH_DIR.\nsource_path must be absolute. dest_path must be a file path in SESSION_SCRATCH_DIR, not a directory.",
            json!({
                "type": "object",
                "properties": {
                    "source_path": { "type": "string", "description": "Absolute readable file." },
                    "dest_path": { "type": "string", "description": "Absolute file path." }
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
