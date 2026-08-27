//! Host file tools (grep / read / write / edit). Paths are checked against a PathFence only.

use std::fs;
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

use super::mcp_client::{ToolCatalog, ToolResult};
use super::path_fence::{require_absolute, PathFence};

const READ_DEFAULT_LIMIT: usize = 2000;
const GREP_MAX_MATCHES: usize = 80;
const GREP_MAX_FILES: usize = 2000;
const GREP_MAX_FILE_BYTES: u64 = 1_000_000;

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
    ])
}

pub fn is_builtin(name: &str) -> bool {
    matches!(name, "grep" | "read" | "write" | "edit")
}

pub fn call(name: &str, arguments: &Value, fence: &PathFence) -> ToolResult {
    let result = match name {
        "grep" => grep(arguments, fence),
        "read" => read(arguments, fence),
        "write" => write(arguments, fence),
        "edit" => edit(arguments, fence),
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

fn arg_str(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing {key}"))
}

fn grep(arguments: &Value, fence: &PathFence) -> Result<String, String> {
    let pattern = arg_str(arguments, "pattern")?;
    let regex = Regex::new(&pattern).map_err(|e| format!("invalid pattern: {e}"))?;
    let roots = match arguments.get("path").and_then(Value::as_str) {
        Some(path) => {
            let path = require_absolute(path)?;
            if !fence.allows_read(&path) {
                return Err("path is outside the read fence".into());
            }
            vec![path]
        }
        None => fence.read_allow.clone(),
    };
    let mut matches = Vec::new();
    let mut files_seen = 0usize;
    for root in roots {
        walk_grep(&root, fence, &regex, &mut matches, &mut files_seen);
        if matches.len() >= GREP_MAX_MATCHES || files_seen >= GREP_MAX_FILES {
            break;
        }
    }
    if matches.is_empty() {
        return Ok("No matches.".into());
    }
    Ok(matches.join("\n"))
}

fn walk_grep(
    root: &Path,
    fence: &PathFence,
    regex: &Regex,
    matches: &mut Vec<String>,
    files_seen: &mut usize,
) {
    if matches.len() >= GREP_MAX_MATCHES || *files_seen >= GREP_MAX_FILES {
        return;
    }
    if root.is_file() {
        grep_file(root, fence, regex, matches, files_seen);
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if matches.len() >= GREP_MAX_MATCHES || *files_seen >= GREP_MAX_FILES {
            return;
        }
        let path = entry.path();
        if !fence.allows_read(&path) {
            continue;
        }
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            walk_grep(&path, fence, regex, matches, files_seen);
        } else if path.is_file() {
            grep_file(&path, fence, regex, matches, files_seen);
        }
    }
}

fn grep_file(
    path: &Path,
    fence: &PathFence,
    regex: &Regex,
    matches: &mut Vec<String>,
    files_seen: &mut usize,
) {
    if !fence.allows_read(path) {
        return;
    }
    *files_seen += 1;
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() > GREP_MAX_FILE_BYTES {
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for (idx, line) in text.lines().enumerate() {
        if matches.len() >= GREP_MAX_MATCHES {
            return;
        }
        if regex.is_match(line) {
            matches.push(format!("{}:{}:{line}", path.display(), idx + 1));
        }
    }
}

fn read(arguments: &Value, fence: &PathFence) -> Result<String, String> {
    let path = require_absolute(&arg_str(arguments, "path")?)?;
    if !fence.allows_read(&path) {
        return Err("path is outside the read fence".into());
    }
    if !path.is_file() {
        return Err("file not found".into());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let offset = arguments
        .get("offset")
        .and_then(Value::as_u64)
        .map(|n| n.max(1) as usize)
        .unwrap_or(1);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(READ_DEFAULT_LIMIT)
        .max(1);
    let lines: Vec<&str> = text.lines().collect();
    let start = offset.saturating_sub(1).min(lines.len());
    let end = start.saturating_add(limit).min(lines.len());
    if start >= end {
        return Ok("File is empty.".into());
    }
    let body = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}:{line}", start + i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(body)
}

fn write(arguments: &Value, fence: &PathFence) -> Result<String, String> {
    let path = require_absolute(&arg_str(arguments, "path")?)?;
    if !fence.allows_write(&path) {
        return Err("path is outside the write fence".into());
    }
    let content = arg_str(arguments, "content")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(format!("Wrote {}", path.display()))
}

fn edit(arguments: &Value, fence: &PathFence) -> Result<String, String> {
    let path = require_absolute(&arg_str(arguments, "path")?)?;
    if !fence.allows_write(&path) {
        return Err("path is outside the write fence".into());
    }
    if !path.is_file() {
        return Err("file not found".into());
    }
    let old_text = arg_str(arguments, "old_text")?;
    let new_text = arg_str(arguments, "new_text")?;
    if old_text.is_empty() {
        return Err("old_text must not be empty".into());
    }
    let current = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let count = current.matches(&old_text).count();
    if count == 0 {
        return Err("old_text not found".into());
    }
    if count > 1 {
        return Err("old_text matched more than once".into());
    }
    let next = current.replacen(&old_text, &new_text, 1);
    fs::write(&path, next).map_err(|e| e.to_string())?;
    Ok(format!("Edited {}", path.display()))
}

#[cfg(test)]
#[path = "../../unit-tests/services/agent/fs_tools_tests.rs"]
mod tests;
