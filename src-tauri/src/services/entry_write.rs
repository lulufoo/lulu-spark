//! Notes Markdown entry writes (port `server.py` `_handle_save`).

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::docs::attached::log_orphan_attachments;
use crate::services::docs::workspace_root;
use crate::services::docs::doc_file_path;
use crate::services::search_index;
use crate::services::spark_data::{self, find_doc};

const EDITABLE_LAYERS: &[&str] = &["raw", "digest"];

pub fn save_entry(
    _repo_root: &Path,
    layer: String,
    id: String,
    content: String,
    lang: Option<String>,
) -> Value {
    let layer = layer.trim();
    if !EDITABLE_LAYERS.contains(&layer) {
        return json!({
            "error": format!("Invalid layer: {layer}"),
            "_status": 400
        });
    }
    let id = id.trim();
    if !spark_data::is_valid_doc_id(id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }

    let conn = match spark_data::open() {
        Ok(c) => c,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let found = match find_doc(&conn, id) {
        Ok(Some(found)) => found,
        Ok(None) => return json!({ "error": "File not found", "_status": 404 }),
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let (workspace, note) = (found.workspace, found.doc);
    let rel = match (layer, lang.as_deref().map(str::trim).filter(|s| !s.is_empty())) {
        ("raw", Some(lang)) => match expected_lang_common_path(&note.raw_path, lang) {
            Some(p) => p,
            None => return json!({ "error": "Invalid path", "_status": 400 }),
        },
        ("digest", _) => note.digest_path.clone().unwrap_or_else(|| note.raw_path.clone()),
        _ => note.raw_path.clone(),
    };
    if rel.is_empty() || rel.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }

    let root = match workspace_root(&workspace) {
        Ok(root) => root,
        Err(v) => return v,
    };
    let target = match doc_file_path(&root, layer, &note.project_id, &rel) {
        Ok(p) => p,
        Err(v) => return v,
    };

    if !target.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{}/{rel}", note.project_id),
            "_status": 404
        });
    }

    if let Err(e) = fs::write(&target, content) {
        return json!({ "error": e.to_string(), "_status": 500 });
    }
    log_orphan_attachments(&root, &note.project_id, &note);
    search_index::sync_workspace_docs_best_effort(&workspace.id, &[id]);
    json!({ "ok": true })
}

#[cfg(test)]
#[path = "../unit-tests/services/entry_write.rs"]
mod tests;
