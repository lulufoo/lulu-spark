//! Notes Markdown entry writes (port `server.py` `_handle_save`).

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::config::roots::notes_root_path;
use crate::services::keyword_index;

const EDITABLE_LAYERS: &[&str] = &["raw", "digest"];

pub fn save_entry(
    repo_root: &Path,
    layer: String,
    common_path: String,
    content: String,
) -> Value {
    let layer = layer.trim();
    if !EDITABLE_LAYERS.contains(&layer) {
        return json!({
            "error": format!("Invalid layer: {layer}"),
            "_status": 400
        });
    }
    let common_path = common_path.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }

    let notes = notes_root_path(repo_root);
    let target = notes.join(layer).join(common_path);
    let notes_canon = match notes.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    let target_canon = target.canonicalize().unwrap_or(target.clone());
    let prefix = format!(
        "{}{}",
        notes_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return json!({ "error": "Path traversal not allowed", "_status": 400 });
    }

    if !target_canon.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{common_path}"),
            "_status": 404
        });
    }

    if let Err(e) = fs::write(&target_canon, content) {
        return json!({ "error": e.to_string(), "_status": 500 });
    }
    keyword_index::sync_note_files_best_effort(repo_root, &[(layer, common_path)]);
    json!({ "ok": true })
}

#[cfg(test)]
#[path = "../unit-tests/services/entry_write.rs"]
mod tests;
