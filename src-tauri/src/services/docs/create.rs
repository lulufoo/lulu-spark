//! `create_doc` / `create_doc_content`: write a new document into `workspace` + `project`.
//!
//! Same rules as the old `create_note`: raw file, optional translations and digest, optional
//! companion assets, then the table row. Files are rolled back when a later step fails.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::docs::assets::{copy_assets, reject_unsupported_asset_paths, resolve_create_assets};
use crate::services::docs::create_meta::parse_create_meta_unchecked;
use crate::services::docs::digest::{
    digest_body_from_payload, digest_body_rejects_relative_images, digest_should_write,
    parse_digest_mode, write_digest_file,
};
use crate::services::docs::store::{
    doc_file_path, parse_translations, reject_legacy_translation_fields,
    resolve_archive_source_markdown, rollback_written, write_markdown_atomic,
};
use crate::services::search_index;
use crate::services::spark_data::{insert_doc, new_doc_id, DocRow};
use crate::services::translation_gate;

use super::resolve::{
    bad_request, conflict, internal, open_data, resolve_project, resolve_workspace, text_field,
    workspace_root,
};

/// Body from an allow-listed `source_path`.
pub fn create_doc(payload: &Value) -> Value {
    if payload.get("document").is_some() {
        return bad_request("document is not supported; use source_path");
    }
    let Some(source_path) = payload.get("source_path").and_then(Value::as_str) else {
        return bad_request("Missing source_path");
    };
    let document = match resolve_archive_source_markdown(source_path) {
        Ok(s) => s,
        Err(v) => return v,
    };
    if document.trim().is_empty() {
        return bad_request("Source file is empty");
    }
    let basename = Path::new(source_path).file_name().and_then(|s| s.to_str());
    write_doc(&document, payload, basename)
}

/// Markdown body in `content`. No `source_path`.
pub fn create_doc_content(payload: &Value) -> Value {
    if payload.get("source_path").is_some() {
        return bad_request("source_path is not supported; use content");
    }
    if payload.get("document").is_some() {
        return bad_request("document is not supported; use content");
    }
    if let Some(err) = reject_unsupported_asset_paths(payload) {
        return err;
    }
    let Some(content) = payload.get("content").and_then(Value::as_str) else {
        return bad_request("Missing content");
    };
    if content.trim().is_empty() {
        return bad_request("Content is empty");
    }
    write_doc(content, payload, None)
}

fn write_doc(source: &str, payload: &Value, source_basename: Option<&str>) -> Value {
    let meta = match parse_create_meta_unchecked(payload, source_basename) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let digest_mode = match parse_digest_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let want_digest = digest_should_write(digest_mode, source, &meta.source_type);
    let digest_body = digest_body_from_payload(payload);
    if want_digest && digest_body.is_none() {
        return bad_request("Missing digest_body");
    }
    if let Some(err) = digest_body.as_deref().filter(|_| want_digest).and_then(digest_body_rejects_relative_images) {
        return err;
    }
    if let Some(err) = reject_legacy_translation_fields(payload) {
        return err;
    }
    let translations = match parse_translations(payload, &meta.common_path, source) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if translation_gate::is_full_english(source) && !translations.iter().any(|t| t.lang == "zh") {
        return bad_request("full English note requires translations.zh");
    }

    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let workspace = match resolve_workspace(&conn, text_field(payload, "workspace")) {
        Ok(w) => w,
        Err(v) => return v,
    };
    let project = match resolve_project(&conn, &workspace, &meta.project) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let root = match workspace_root(&workspace) {
        Ok(r) => r,
        Err(v) => return v,
    };

    let folder = project.folder.as_str();
    let raw_path = match doc_file_path(&root, "raw", folder, &meta.common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if raw_path.is_file() {
        return conflict("File already exists");
    }
    let mut extra_paths: Vec<PathBuf> = Vec::new();
    for t in &translations {
        let path = match doc_file_path(&root, "raw", folder, &t.common_path) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if path.is_file() {
            return conflict("File already exists");
        }
        extra_paths.push(path);
    }
    let assets = match resolve_create_assets(payload, &raw_path, &meta.common_path) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let mut written: Vec<PathBuf> = Vec::new();
    if let Err(e) = write_markdown_atomic(&raw_path, source) {
        return internal(e);
    }
    written.push(raw_path);
    for (t, path) in translations.iter().zip(extra_paths.iter()) {
        if let Err(e) = write_markdown_atomic(path, &t.content) {
            rollback_written(&written);
            return internal(e);
        }
        written.push(path.clone());
    }
    let mut digest_rel: Option<String> = None;
    if let Some(digest_md) = digest_body.as_deref().filter(|_| want_digest) {
        match write_digest_file(&root, folder, &meta.common_path, digest_md, &mut written) {
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

    let id = new_doc_id(&workspace.id);
    let mut row = DocRow::new(&id, &meta.common_path, &project.id);
    row.title = Some(meta.title.clone());
    row.created_at = Some(meta.created_at.clone());
    row.source_type = Some(meta.source_type.clone());
    row.digest_path = digest_rel.as_ref().map(|_| meta.common_path.clone());
    row.translations = json!(translations.iter().map(|t| t.lang.as_str()).collect::<Vec<_>>());
    if let Err(e) = insert_doc(&conn, &workspace.doc_table, &row) {
        rollback_written(&written);
        return internal(e);
    }

    search_index::sync_workspace_docs_best_effort(&workspace.id, &[id.as_str()]);

    let mut response = json!({
        "ok": true,
        "id": id,
        "workspace": workspace.id,
        "workspace_name": workspace.name,
        "project": project.folder,
        "created_at": meta.created_at,
    });
    if !translations.is_empty() {
        response["extra_paths"] = json!(translations.iter().map(|t| t.rel.as_str()).collect::<Vec<_>>());
    }
    if let Some(rel) = digest_rel {
        response["digest_path"] = json!(rel);
    }
    if !assets.is_empty() {
        response["asset_paths"] = json!(assets.iter().map(|a| a.rel.as_str()).collect::<Vec<_>>());
    }
    response
}
