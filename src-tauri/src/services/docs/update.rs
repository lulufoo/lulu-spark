//! `update_doc` / `update_doc_content`: overwrite an existing document body (and optional
//! digest) by id, inside one workspace.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::services::docs::digest::{
    digest_body_from_payload, digest_should_write, parse_digest_mode, write_digest_file_overwrite,
};
use crate::services::docs::store::{
    doc_file_path, reject_legacy_translation_fields, resolve_archive_source_markdown,
    write_markdown_atomic,
};
use crate::services::search_index;
use crate::services::spark_data::{update_doc as update_doc_row, DocRow};
use crate::services::translation_gate;

use super::resolve::{
    bad_request, doc_id_field, internal, not_found, open_data, resolve_doc, resolve_doc_workspace,
    workspace_root,
};

/// Fields that only `create_doc` accepts.
const DEFERRED_FIELDS: &[&str] = &[
    "title",
    "project",
    "folder",
    "theme",
    "created_at",
    "source_type",
    "translations",
    "asset_paths",
];

/// Body from an allow-listed `source_path`.
pub fn update_doc(payload: &Value) -> Value {
    if payload.get("document").is_some() {
        return bad_request("document is not supported; use source_path");
    }
    if payload.get("content").is_some() {
        return bad_request("content is not supported; use source_path");
    }
    let Some(source_path) = payload.get("source_path").and_then(Value::as_str) else {
        return bad_request("Missing source_path");
    };
    match resolve_archive_source_markdown(source_path) {
        Ok(document) => update_from_body(payload, &document),
        Err(v) => v,
    }
}

/// Markdown body in `content`.
pub fn update_doc_content(payload: &Value) -> Value {
    if payload.get("source_path").is_some() {
        return bad_request("source_path is not supported; use content");
    }
    if payload.get("document").is_some() {
        return bad_request("document is not supported; use content");
    }
    let Some(content) = payload.get("content").and_then(Value::as_str) else {
        return bad_request("Missing content");
    };
    update_from_body(payload, content)
}

fn update_from_body(payload: &Value, source: &str) -> Value {
    for key in DEFERRED_FIELDS {
        if payload.get(*key).is_some() {
            return bad_request(format!("{key} is not supported on update_doc"));
        }
    }
    if let Some(err) = reject_legacy_translation_fields(payload) {
        return err;
    }
    let id = match doc_id_field(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let digest_mode = match parse_digest_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    if source.trim().is_empty() {
        return bad_request("Content is empty");
    }

    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let workspace = match resolve_doc_workspace(&conn, payload, &id) {
        Ok(w) => w,
        Err(v) => return v,
    };
    let (doc, project) = match resolve_doc(&conn, &workspace, &id) {
        Ok(found) => found,
        Err(v) => return v,
    };
    let rel = doc.raw_path.trim().to_string();
    if rel.is_empty() || rel.contains("..") {
        return bad_request("Invalid path");
    }
    let source_type = doc.source_type.clone().unwrap_or_else(|| "summary".to_string());
    if translation_gate::is_full_english(source) && !has_zh(&doc) {
        return bad_request("full English note requires translations.zh");
    }
    let want_digest = digest_should_write(digest_mode, source, &source_type);
    let digest_body = digest_body_from_payload(payload);
    if want_digest && digest_body.is_none() {
        return bad_request("Missing digest_body");
    }

    let root = match workspace_root(&workspace) {
        Ok(r) => r,
        Err(v) => return v,
    };
    let raw_path = match doc_file_path(&root, "raw", &project.folder, &rel) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !raw_path.is_file() {
        return not_found("File not found");
    }
    let prev_raw = match fs::read_to_string(&raw_path) {
        Ok(s) => s,
        Err(e) => return internal(e.to_string()),
    };
    if let Err(e) = write_markdown_atomic(&raw_path, source) {
        return internal(e);
    }

    let mut digest_rel: Option<String> = None;
    if let Some(digest_md) = digest_body.as_deref().filter(|_| want_digest) {
        match write_digest_file_overwrite(&root, &project.folder, &rel, digest_md) {
            Ok(rel) => digest_rel = Some(rel),
            Err(v) => return restore_raw(&raw_path, &prev_raw, v),
        }
    }
    if digest_rel.is_some() && doc.digest_path.is_none() {
        let mut updated = doc.clone();
        updated.digest_path = Some(rel.clone());
        if let Err(e) = update_doc_row(&conn, &workspace.doc_table, &updated) {
            return restore_raw(&raw_path, &prev_raw, internal(e));
        }
    }

    search_index::sync_workspace_docs_best_effort(&workspace.id, &[id.as_str()]);

    let mut response = json!({ "ok": true, "id": id, "workspace": workspace.id });
    if let Some(rel) = digest_rel {
        response["digest_path"] = json!(rel);
    }
    response
}

fn restore_raw(raw_path: &Path, previous: &str, error: Value) -> Value {
    let _ = write_markdown_atomic(raw_path, previous);
    error
}

fn has_zh(doc: &DocRow) -> bool {
    doc.translations
        .as_array()
        .is_some_and(|langs| langs.iter().any(|l| l.as_str() == Some("zh")))
}
