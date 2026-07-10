//! Draft cache under `{cache_dir}/drafts/` (comment drafts) and
//! `{cache_dir}/drafts/notes/<temp_id>` (note create crash buffer).

use std::fs;

use serde_json::{json, Value};

use crate::config::paths;

pub fn save_comment_draft(payload: &Value) -> Value {
    let common_path = payload
        .get("common_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let draft_file = match paths::draft_path(common_path) {
        Ok(p) => p,
        Err(_) => return json!({ "error": "Path traversal not allowed", "_status": 400 }),
    };
    if !content.is_empty() {
        if let Some(parent) = draft_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return json!({ "error": e.to_string(), "_status": 500 });
            }
        }
        if let Err(e) = fs::write(&draft_file, content) {
            return json!({ "error": e.to_string(), "_status": 500 });
        }
    } else if draft_file.exists() {
        let _ = fs::remove_file(&draft_file);
    }
    json!({ "ok": true })
}

/// Write note create crash-buffer content under `drafts/notes/<temp_id>`.
pub fn save_note_draft(temp_id: &str, content: &str) -> Result<(), String> {
    let draft_file = paths::notes_draft_path(temp_id).map_err(|_| "Invalid path".to_string())?;
    if let Some(parent) = draft_file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&draft_file, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove note create crash-buffer file (success or empty-exit cleanup).
pub fn clear_note_draft(temp_id: &str) -> Result<(), String> {
    let draft_file = paths::notes_draft_path(temp_id).map_err(|_| "Invalid path".to_string())?;
    if draft_file.exists() {
        fs::remove_file(&draft_file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../unit-tests/services/draft.rs"]
mod tests;
