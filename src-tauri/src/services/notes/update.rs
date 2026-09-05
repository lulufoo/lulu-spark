//! Overwrite an existing note body (and optional digest) by archive entry id.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::config::roots::notes_root_path;
use crate::services::keyword_index;
use crate::services::translation_gate;

use super::digest::{
    digest_body_from_payload, digest_should_write, parse_digest_mode, write_digest_file_overwrite,
};
use super::store::{
    load_entries_map, notes_layer_path, reject_legacy_translation_fields,
    resolve_archive_source_markdown, save_entries, write_markdown_atomic,
};

const DEFERRED_FIELDS: &[&str] = &[
    "title",
    "project",
    "theme",
    "created_at",
    "source_type",
    "translations",
];

/// Public note API (HTTP / MCP): body from allow-listed `source_path` only.
pub fn update_note(repo_root: &Path, payload: &Value) -> Value {
    if payload.get("document").is_some() {
        return json!({
            "error": "document is not supported; use source_path",
            "_status": 400
        });
    }
    if payload.get("content").is_some() {
        return json!({
            "error": "content is not supported; use source_path",
            "_status": 400
        });
    }
    let Some(source_path) = payload.get("source_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing source_path", "_status": 400 });
    };
    let document = match resolve_archive_source_markdown(source_path) {
        Ok(s) => s,
        Err(v) => return v,
    };
    update_note_from_body(repo_root, payload, &document)
}

/// Public note API (HTTP / MCP mobile): Markdown body in `content`.
pub fn update_note_content(repo_root: &Path, payload: &Value) -> Value {
    if payload.get("source_path").is_some() {
        return json!({
            "error": "source_path is not supported; use content",
            "_status": 400
        });
    }
    if payload.get("document").is_some() {
        return json!({
            "error": "document is not supported; use content",
            "_status": 400
        });
    }
    let Some(content) = payload.get("content").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing content", "_status": 400 });
    };
    update_note_from_body(repo_root, payload, content)
}

fn update_note_from_body(repo_root: &Path, payload: &Value, source: &str) -> Value {
    if let Some(err) = reject_deferred_fields(payload) {
        return err;
    }
    if let Some(err) = reject_legacy_translation_fields(payload) {
        return err;
    }
    let id = match parse_entry_id(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let digest_mode = match parse_digest_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    if source.trim().is_empty() {
        return json!({ "error": "Content is empty", "_status": 400 });
    }

    let (index_path, mut entries) = match load_entries_map(repo_root) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let Some(entry) = entries.get(&id).cloned() else {
        return json!({ "error": "Entry not found", "_status": 404 });
    };
    let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing common_path", "_status": 400 });
    };
    let common_path = common_path.trim().to_string();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid common_path", "_status": 400 });
    }
    let source_type = entry
        .get("source_type")
        .and_then(|v| v.as_str())
        .unwrap_or("summary")
        .to_string();

    if translation_gate::is_full_english(source) && !entry_has_zh(&entry) {
        return json!({
            "error": "full English note requires translations.zh",
            "_status": 400
        });
    }

    let want_digest = digest_should_write(digest_mode, source, &source_type);
    let digest_body = digest_body_from_payload(payload);
    if want_digest && digest_body.is_none() {
        return json!({ "error": "Missing digest_body", "_status": 400 });
    }

    let notes = notes_root_path(repo_root);
    let raw_path = match notes_layer_path(&notes, "raw", &common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !raw_path.is_file() {
        return json!({
            "error": format!("File not found: raw/{common_path}"),
            "_status": 404
        });
    }

    let prev_raw = match fs::read_to_string(&raw_path) {
        Ok(s) => s,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    if let Err(e) = write_markdown_atomic(&raw_path, source) {
        return json!({ "error": e, "_status": 500 });
    }

    let mut digest_rel: Option<String> = None;
    if want_digest {
        let Some(digest_md) = digest_body.as_deref() else {
            let _ = write_markdown_atomic(&raw_path, &prev_raw);
            return json!({ "error": "Missing digest_body", "_status": 400 });
        };
        match write_digest_file_overwrite(&notes, &common_path, digest_md) {
            Ok(rel) => digest_rel = Some(rel),
            Err(v) => {
                let _ = write_markdown_atomic(&raw_path, &prev_raw);
                return v;
            }
        }
    }

    if digest_rel.is_some() {
        if let Some(updated) = with_digest_layer(entry) {
            entries.insert(id.clone(), updated);
            if let Err(v) = save_entries(&index_path, &entries) {
                let _ = write_markdown_atomic(&raw_path, &prev_raw);
                return v;
            }
        }
    }

    let mut synced: Vec<(&str, &str)> = vec![("raw", common_path.as_str())];
    if digest_rel.is_some() {
        synced.push(("digest", common_path.as_str()));
    }
    keyword_index::sync_note_files_best_effort(repo_root, &synced);

    let mut response = json!({
        "ok": true,
        "id": id,
        "common_path": common_path,
        "raw_path": format!("raw/{common_path}"),
    });
    if let Some(rel) = digest_rel {
        response["digest_path"] = json!(rel);
    }
    response
}

fn parse_entry_id(payload: &Value) -> Result<String, Value> {
    let Some(id) = payload.get("id").and_then(|v| v.as_str()).map(str::trim) else {
        return Err(json!({ "error": "Missing id", "_status": 400 }));
    };
    if id.len() != 32 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(json!({ "error": "Invalid id", "_status": 400 }));
    }
    Ok(id.to_string())
}

fn reject_deferred_fields(payload: &Value) -> Option<Value> {
    for key in DEFERRED_FIELDS {
        if payload.get(*key).is_some() {
            return Some(json!({
                "error": format!("{key} is not supported on update_note"),
                "_status": 400
            }));
        }
    }
    None
}

fn entry_has_zh(entry: &Value) -> bool {
    entry.get("translations").and_then(|t| t.get("zh")).is_some()
}

fn with_digest_layer(mut entry: Value) -> Option<Value> {
    match entry.get_mut("layers").and_then(|v| v.as_array_mut()) {
        Some(layers) => {
            if layers.iter().any(|v| v.as_str() == Some("digest")) {
                return None;
            }
            layers.push(json!("digest"));
        }
        None => {
            entry["layers"] = json!(["raw", "digest"]);
        }
    }
    Some(entry)
}