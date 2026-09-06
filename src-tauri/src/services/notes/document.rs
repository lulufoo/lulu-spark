use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::config::roots::notes_root_path;
use crate::services::id::random_entry_id;
use crate::services::keyword_index;
use crate::services::translation_gate;

use super::assets::{copy_assets, reject_unsupported_asset_paths, resolve_create_assets};
use super::create_meta::{assemble_raw, body_from_source, parse_create_meta};
use super::digest::{
    digest_body_from_payload, digest_body_rejects_relative_images, digest_should_write,
    parse_digest_mode, write_digest_file,
};
use super::store::{
    dual_store_rollback, load_entries_map, notes_layer_path, parse_translations,
    reject_legacy_translation_fields, resolve_archive_source_markdown, rollback_written,
    save_entries, write_markdown_atomic,
};

/// Public note API (HTTP / MCP): body from allow-listed `source_path` only.
pub fn create_note(repo_root: &Path, payload: &Value) -> Value {
    if payload.get("document").is_some() {
        return json!({
            "error": "document is not supported; use source_path",
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
    if document.trim().is_empty() {
        return json!({ "error": "Source file is empty", "_status": 400 });
    }
    let basename = std::path::Path::new(source_path)
        .file_name()
        .and_then(|s| s.to_str());
    write_note(repo_root, &document, payload, basename)
}

/// Public note API (HTTP / MCP mobile): Markdown body in `content`. No `source_path`.
pub fn create_note_content(repo_root: &Path, payload: &Value) -> Value {
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
    if let Some(err) = reject_unsupported_asset_paths(payload) {
        return err;
    }
    let Some(content) = payload.get("content").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing content", "_status": 400 });
    };
    if content.trim().is_empty() {
        return json!({ "error": "Content is empty", "_status": 400 });
    }
    write_note(repo_root, content, payload, None)
}

/// Internal write path used by jot and tests that already hold markdown.
pub(super) fn create_note_from_markdown(
    repo_root: &Path,
    document: &str,
    payload: &Value,
) -> Value {
    write_note(repo_root, document, payload, None)
}

fn write_note(
    repo_root: &Path,
    source: &str,
    payload: &Value,
    source_basename: Option<&str>,
) -> Value {
    let meta = match parse_create_meta(payload, source_basename) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let digest_mode = match parse_digest_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let body = body_from_source(source);
    let raw_doc = match assemble_raw(&meta.title, &meta.created_at, &body) {
        Ok(s) => s,
        Err(v) => return v,
    };
    let want_digest = digest_should_write(digest_mode, &raw_doc, &meta.source_type);
    let digest_body = digest_body_from_payload(payload);
    if want_digest && digest_body.is_none() {
        return json!({ "error": "Missing digest_body", "_status": 400 });
    }
    if want_digest {
        if let Some(digest_md) = digest_body.as_deref() {
            if let Some(err) = digest_body_rejects_relative_images(digest_md) {
                return err;
            }
        }
    }

    if let Some(err) = reject_legacy_translation_fields(payload) {
        return err;
    }
    let translations = match parse_translations(payload, &meta.common_path, source) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if translation_gate::is_full_english(&raw_doc)
        && !translations.iter().any(|t| t.lang == "zh")
    {
        return json!({
            "error": "full English note requires translations.zh",
            "_status": 400
        });
    }

    let notes = notes_root_path(repo_root);
    let raw_path = match notes_layer_path(&notes, "raw", &meta.common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if raw_path.is_file() {
        return json!({
            "error": format!("File already exists: raw/{}", meta.common_path),
            "_status": 409
        });
    }

    let mut extra_paths: Vec<PathBuf> = Vec::new();
    for t in &translations {
        let path = match notes_layer_path(&notes, "raw", &t.common_path) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if path.is_file() {
            return json!({
                "error": format!("File already exists: {}", t.rel),
                "_status": 409
            });
        }
        extra_paths.push(path);
    }

    let assets = match resolve_create_assets(payload, &raw_path, &meta.common_path) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let mut written: Vec<PathBuf> = Vec::new();
    if let Err(e) = write_markdown_atomic(&raw_path, &raw_doc) {
        return json!({ "error": e, "_status": 500 });
    }
    written.push(raw_path);

    for (t, path) in translations.iter().zip(extra_paths.iter()) {
        if let Err(e) = write_markdown_atomic(path, &t.content) {
            rollback_written(&written);
            return json!({ "error": e, "_status": 500 });
        }
        written.push(path.clone());
    }

    let mut digest_rel: Option<String> = None;
    if want_digest {
        let Some(digest_md) = digest_body.as_deref() else {
            rollback_written(&written);
            return json!({ "error": "Missing digest_body", "_status": 400 });
        };
        match write_digest_file(&notes, &meta.common_path, digest_md, &mut written) {
            Ok(rel) => digest_rel = Some(rel),
            Err(v) => {
                rollback_written(&written);
                return v;
            }
        }
    }

    if let Err(v) = copy_assets(&assets, &mut written) {
        rollback_written(&written);
        return v;
    }

    let id = random_entry_id();
    let (index_path, mut entries) = match load_entries_map(repo_root) {
        Ok(v) => v,
        Err(v) => {
            rollback_written(&written);
            return v;
        }
    };
    let index_snapshot = entries.clone();

    let mut entry = Map::new();
    entry.insert("common_path".to_string(), json!(meta.common_path));
    entry.insert("created_at".to_string(), json!(meta.created_at));
    let layers = if digest_rel.is_some() {
        json!(["raw", "digest"])
    } else {
        json!(["raw"])
    };
    entry.insert("layers".to_string(), layers);
    entry.insert("source_type".to_string(), json!(meta.source_type));
    if !translations.is_empty() {
        let mut map = Map::new();
        for t in &translations {
            map.insert(t.lang.clone(), json!(t.common_path));
        }
        entry.insert("translations".to_string(), Value::Object(map));
    }
    entries.insert(id.clone(), Value::Object(entry));

    if let Err(v) = save_entries(&index_path, &entries) {
        dual_store_rollback(&written, &index_path, &index_snapshot);
        return v;
    }

    let mut synced: Vec<(&str, &str)> = vec![("raw", meta.common_path.as_str())];
    if digest_rel.is_some() {
        synced.push(("digest", meta.common_path.as_str()));
    }
    keyword_index::sync_note_files_best_effort(repo_root, &synced);

    let extra_rel_paths: Vec<String> = translations.iter().map(|t| t.rel.clone()).collect();
    let mut response = json!({
        "ok": true,
        "id": id,
        "common_path": meta.common_path,
        "raw_path": format!("raw/{}", meta.common_path),
        "created_at": meta.created_at
    });
    if !extra_rel_paths.is_empty() {
        response["extra_paths"] = json!(extra_rel_paths);
    }
    if let Some(rel) = digest_rel {
        response["digest_path"] = json!(rel);
    }
    if !assets.is_empty() {
        let rels: Vec<&str> = assets.iter().map(|a| a.rel.as_str()).collect();
        response["asset_paths"] = json!(rels);
    }
    response
}
