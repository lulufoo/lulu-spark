//! `move_doc`: move a document to another project of the same workspace.
//!
//! Files move first (all or nothing), then the row.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::docs::store::doc_file_path;
use crate::services::search_index;
use crate::services::spark_data::{translation_common_paths, update_doc as update_doc_row};

use super::resolve::{
    bad_request, doc_id_field, ensure_workspace_enabled, internal, open_data,
    resolve_doc, resolve_doc_workspace, resolve_project, text_field, workspace_root,
};

const LAYERS: &[&str] = &["raw", "digest"];

fn rollback(completed: &[(PathBuf, PathBuf)]) {
    for (src, dst) in completed.iter().rev() {
        let _ = fs::rename(dst, src);
    }
}

/// `{ workspace_id?, id, project_id }` →
/// `{ ok, id, workspace_id, workspace_name, project_id, project_name }`.
pub fn move_doc(payload: &Value) -> Value {
    let id = match doc_id_field(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let Some(target) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let workspace = match resolve_doc_workspace(&conn, payload, &id) {
        Ok(w) => w,
        Err(v) => return v,
    };
    let (mut doc, old_project) = match resolve_doc(&conn, &workspace, &id) {
        Ok(found) => found,
        Err(v) => return v,
    };
    let new_project = match resolve_project(&conn, &workspace, Some(target)) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if let Err(v) = ensure_workspace_enabled(&workspace) {
        return v;
    }
    if new_project.id == old_project.id {
        return bad_request("Already in this project");
    }
    let rel = doc.raw_path.clone();
    if rel.is_empty() || rel.contains("..") {
        return bad_request("Invalid path");
    }
    let root = match workspace_root(&workspace) {
        Ok(r) => r,
        Err(v) => return v,
    };

    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    for layer in LAYERS {
        let src = match doc_file_path(&root, layer, &old_project.id, &rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        let dst = match doc_file_path(&root, layer, &new_project.id, &rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if src.exists() {
            moves.push((src, dst));
        }
    }
    let translations = match translation_common_paths(&doc) {
        Ok(paths) => paths,
        Err(e) => return internal(e),
    };
    for (lang, old_rel) in &translations {
        let Some(new_rel) = expected_lang_common_path(&rel, lang) else {
            return bad_request("Invalid path");
        };
        let src = match doc_file_path(&root, "raw", &old_project.id, old_rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        let dst = match doc_file_path(&root, "raw", &new_project.id, &new_rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if src.exists() {
            moves.push((src, dst));
        }
    }

    let mut completed: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (src, dst) in &moves {
        if let Some(parent) = dst.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                rollback(&completed);
                return internal(format!("Move failed, rolled back: {e}"));
            }
        }
        if let Err(e) = fs::rename(src, dst) {
            rollback(&completed);
            return internal(format!("Move failed, rolled back: {e}"));
        }
        completed.push((src.clone(), dst.clone()));
    }

    doc.project_id = new_project.id.clone();
    if let Err(e) = update_doc_row(&conn, &doc) {
        rollback(&completed);
        return internal(e);
    }
    search_index::sync_workspace_docs_best_effort(&workspace.id, &[id.as_str()]);
    json!({
        "ok": true,
        "id": id,
        "workspace_id": workspace.id,
        "workspace_name": workspace.name,
        "project_id": new_project.id,
        "project_name": new_project.name,
    })
}
