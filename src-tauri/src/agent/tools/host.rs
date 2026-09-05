//! Host file tools (grep / read / write / str_replace) under PathFence.

use serde_json::{json, Value};

use crate::services::path_fence::PathFence;

use super::catalog::{LocalTool, ToolCatalog, ToolResult};
use super::fs;

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
            "Read a text file under a read-allowed root. path must be absolute. Optional 1-based offset and limit (default 50 lines). Success is JSON: offset, limit, remaining_lines, and line-numbered content.",
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
            "Create or overwrite a text file. path must be absolute. Writable locations: the session scratch directory {session_scratch}, or an exact currently staged file. Do not invent a sibling filename; use list_staged for staged paths.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path under the session scratch directory or an exact currently staged file." },
                    "content": { "type": "string", "description": "Full file contents." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            false,
        ),
        local_tool(
            "str_replace",
            "Replace exact text in an existing writable file. Writable locations: the session scratch directory {session_scratch}, or an exact currently staged file. Do not invent a sibling filename; use list_staged for staged paths. old_string must match exactly once unless replace_all is true.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path under the session scratch directory or an exact currently staged file." },
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
    ])
}

pub fn is_builtin(name: &str) -> bool {
    matches!(name, "grep" | "read" | "write" | "str_replace")
}

pub fn call(name: &str, arguments: &Value, fence: &PathFence) -> ToolResult {
    let result = match name {
        "grep" => fs::grep(arguments, fence),
        "read" => fs::read(arguments, fence),
        "write" => fs::write(arguments, fence),
        "str_replace" => fs::str_replace(arguments, fence),
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
