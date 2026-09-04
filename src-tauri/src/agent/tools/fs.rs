//! Path-fenced Agent file tool implementations (grep / read / write / edit).

use std::fs as std_fs;
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

use crate::services::path_fence::{require_absolute, PathFence};

const READ_DEFAULT_LIMIT: usize = 50;
const GREP_MAX_MATCHES: usize = 80;
const GREP_MAX_FILES: usize = 2000;
const GREP_MAX_FILE_BYTES: u64 = 1_000_000;

pub fn arg_str(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing {key}"))
}

pub fn grep(arguments: &Value, fence: &PathFence) -> Result<String, String> {
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
    let Ok(entries) = std_fs::read_dir(root) else {
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
    let Ok(meta) = std_fs::metadata(path) else {
        return;
    };
    if meta.len() > GREP_MAX_FILE_BYTES {
        return;
    }
    let Ok(text) = std_fs::read_to_string(path) else {
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

pub fn read(arguments: &Value, fence: &PathFence) -> Result<String, String> {
    let path = require_absolute(&arg_str(arguments, "path")?)?;
    if !fence.allows_read(&path) {
        return Err("path is outside the read fence".into());
    }
    if !path.is_file() {
        return Err("file not found".into());
    }
    let text = std_fs::read_to_string(&path).map_err(|e| e.to_string())?;
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
    let content = if start >= end {
        String::new()
    } else {
        lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}:{line}", start + i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };
    serde_json::to_string(&json!({
        "offset": offset,
        "limit": limit,
        "remaining_lines": lines.len().saturating_sub(end),
        "content": content,
    }))
    .map_err(|e| e.to_string())
}

pub fn write(arguments: &Value, fence: &PathFence) -> Result<String, String> {
    let path = require_absolute(&arg_str(arguments, "path")?)?;
    if !fence.allows_write(&path) {
        return Err("path is outside the write fence".into());
    }
    let content = arg_str(arguments, "content")?;
    if let Some(parent) = path.parent() {
        std_fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std_fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(format!("Wrote {}", path.display()))
}

pub fn edit(arguments: &Value, fence: &PathFence) -> Result<String, String> {
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
    let current = std_fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let count = current.matches(&old_text).count();
    if count == 0 {
        return Err("old_text not found".into());
    }
    if count > 1 {
        return Err("old_text matched more than once".into());
    }
    std_fs::write(&path, current.replacen(&old_text, &new_text, 1)).map_err(|e| e.to_string())?;
    Ok(format!("Edited {}", path.display()))
}
