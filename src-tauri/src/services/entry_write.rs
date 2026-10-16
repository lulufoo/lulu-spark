//! Notes Markdown entry writes (port `server.py` `_handle_save`).

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::config::roots::notes_root_path;
use crate::services::archive_parse::expected_lang_common_path;
use crate::services::notes::doc_file_path;
use crate::services::search_index;
use crate::services::spark_data::{self, category_folder, get_note};

const EDITABLE_LAYERS: &[&str] = &["raw", "digest"];

fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn save_entry(
    repo_root: &Path,
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
    if !is_valid_entry_id(id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }

    let conn = match spark_data::open() {
        Ok(c) => c,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let note = match get_note(&conn, id) {
        Ok(Some(n)) => n,
        Ok(None) => return json!({ "error": "File not found", "_status": 404 }),
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let folder = match category_folder(&conn, &note.category_id) {
        Ok(f) => f,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
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

    let notes = notes_root_path(repo_root);
    let target = match doc_file_path(&notes, layer, &folder, &rel) {
        Ok(p) => p,
        Err(v) => return v,
    };

    if !target.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{folder}/{rel}"),
            "_status": 404
        });
    }

    if let Err(e) = fs::write(&target, content) {
        return json!({ "error": e.to_string(), "_status": 500 });
    }
    search_index::sync_note_docs_best_effort(repo_root, &[id]);
    json!({ "ok": true })
}

#[cfg(test)]
#[path = "../unit-tests/services/entry_write.rs"]
mod tests;
