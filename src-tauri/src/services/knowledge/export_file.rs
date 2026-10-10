//! `get_knowledge_file`: copy a knowledge document into a caller-chosen directory.

use std::path::PathBuf;

use serde_json::{json, Value};

use super::mcp::get_knowledge_path_by_id;
use crate::services::file_export::{copy_into_dir_versioned, copy_result};

/// Copy knowledge document `id` into `dest_dir`, keeping the source extension.
/// Success is `{id, ok, path}` only.
pub fn get_knowledge_file(id: &str, dest_dir: &str) -> Value {
    let located = get_knowledge_path_by_id(id);
    if located.get("ok") != Some(&json!(true)) {
        return located;
    }
    let (Some(doc_id), Some(source)) = (
        located.get("id").and_then(|v| v.as_str()),
        located.get("path").and_then(|v| v.as_str()),
    ) else {
        return json!({ "id": id.trim(), "ok": false, "error": "Missing path" });
    };
    let source = PathBuf::from(source);
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
    copy_result(doc_id, copy_into_dir_versioned(&source, dest_dir, doc_id, ext))
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/export_file.rs"]
mod tests;
