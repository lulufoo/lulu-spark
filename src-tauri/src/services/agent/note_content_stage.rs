//! Host-agent wrap for `get_note_content_by_id`: stage the path, hide it from the model.

use serde_json::{json, Value};

use super::fs_tools;
use super::mcp_client::ToolResult;

pub const NOTE_CONTENT_TOOL: &str = "get_note_content_by_id";

/// After a workbench note-path success, register Stage and return F1/F2… only.
pub fn overlay_tool_result(name: &str, result: ToolResult) -> (ToolResult, bool) {
    if name != NOTE_CONTENT_TOOL || result.is_error {
        return (result, false);
    }
    let Ok(value) = serde_json::from_str::<Value>(&result.content) else {
        return (result, false);
    };
    if value.get("ok") != Some(&Value::Bool(true)) {
        return (result, false);
    }
    let Some(path) = value
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            ToolResult {
                content: "get_note_content_by_id succeeded without a path".into(),
                is_error: true,
            },
            false,
        );
    };
    let note_id = value.get("id").and_then(Value::as_str).unwrap_or("");
    match fs_tools::stage_path(path, None) {
        Ok(entry) => (
            ToolResult {
                content: json!({
                    "ok": true,
                    "id": entry.id,
                    "title": entry.title,
                    "note_id": note_id,
                })
                .to_string(),
                is_error: false,
            },
            true,
        ),
        Err(content) => (ToolResult { content, is_error: true }, false),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/agent/note_content_stage_tests.rs"]
mod tests;
