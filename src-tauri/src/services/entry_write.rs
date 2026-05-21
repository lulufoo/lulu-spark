//! Corpus Markdown entry writes (port `server.py` `_handle_save`).

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::config::meili_env::workbench_knowledge_root_path;

const EDITABLE_LAYERS: &[&str] = &["raw", "distilled", "digest", "trace"];

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

    let corpus = workbench_knowledge_root_path(repo_root);
    let target = corpus.join(layer).join(common_path);
    let corpus_canon = match corpus.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    let target_canon = target.canonicalize().unwrap_or(target.clone());
    let prefix = format!(
        "{}{}",
        corpus_canon.to_string_lossy(),
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
    json!({ "ok": true })
}

#[cfg(test)]
#[path = "../unit-tests/services/entry_write.rs"]
mod tests;
