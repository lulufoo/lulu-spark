//! `move_doc`: move a document to another project, same workspace or another.
//!
//! Plan → stage (copy) → database (`project_id`, commit point) → cleanup → index the
//! target workspace once.

use serde_json::{json, Value};

use crate::services::id::random_alnum6;
use crate::services::search_index;
use crate::services::spark_data::{list_docs, update_doc as update_doc_row};

use super::merge_files::collect_taken_paths;
use super::relocate::{cleanup_relocate, plan_relocate, stage_relocate};
use super::resolve::{
    bad_request, doc_id_field, ensure_workspace_enabled, internal, open_data, resolve_doc,
    resolve_doc_workspace, resolve_project, resolve_workspace, text_field, workspace_root,
};

/// `{ id, project_id, workspace_id?, target_workspace_id? }` →
/// `{ ok, id, workspace_id, workspace_name, project_id, project_name }`.
/// `workspace_id` must be the document's current workspace when sent.
/// `target_workspace_id` defaults to that workspace.
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
    let src_ws = match resolve_doc_workspace(&conn, payload, &id) {
        Ok(w) => w,
        Err(v) => return v,
    };
    let (mut doc, old_project) = match resolve_doc(&conn, &src_ws, &id) {
        Ok(found) => found,
        Err(v) => return v,
    };
    let dest_ws = match text_field(payload, "target_workspace_id") {
        Some(wid) => match resolve_workspace(&conn, Some(wid)) {
            Ok(w) => w,
            Err(v) => return v,
        },
        None => src_ws.clone(),
    };
    if let Err(v) = ensure_workspace_enabled(&src_ws) {
        return v;
    }
    if let Err(v) = ensure_workspace_enabled(&dest_ws) {
        return v;
    }
    let new_project = match resolve_project(&conn, &dest_ws, Some(target)) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if new_project.id == old_project.id {
        return bad_request("Already in this project");
    }
    let rel = doc.raw_path.clone();
    if rel.is_empty() || rel.contains("..") {
        return bad_request("Invalid path");
    }
    let src_root = match workspace_root(&src_ws) {
        Ok(r) => r,
        Err(v) => return v,
    };
    let dest_root = match workspace_root(&dest_ws) {
        Ok(r) => r,
        Err(v) => return v,
    };
    let dest_docs: Vec<_> = match list_docs(&conn, &dest_ws.id) {
        Ok(docs) => docs
            .into_iter()
            .filter(|d| d.project_id == new_project.id)
            .collect(),
        Err(e) => return internal(e),
    };
    let (taken_raw, taken_digest) = match collect_taken_paths(&dest_root, &new_project.id, &dest_docs)
    {
        Ok(v) => v,
        Err(v) => return v,
    };
    let plan = match plan_relocate(
        &doc,
        &new_project.id,
        &taken_raw,
        &taken_digest,
        random_alnum6,
    ) {
        Ok(p) => p,
        Err(e) => return internal(e),
    };
    if let Err(v) = stage_relocate(&src_root, &dest_root, &plan) {
        return v;
    }
    doc.project_id = new_project.id.clone();
    doc.raw_path = plan.target.raw_path.clone();
    if doc.digest_path.is_some() {
        doc.digest_path = plan.target.digest_path.clone();
    }
    match update_doc_row(&conn, &doc) {
        Ok(true) => {}
        Ok(false) => return internal("document missing"),
        Err(e) => return internal(e),
    }
    if let Err(v) = cleanup_relocate(&src_root, &plan) {
        return v;
    }
    search_index::sync_workspace_docs_best_effort(&dest_ws.id, &[id.as_str()]);
    json!({
        "ok": true,
        "id": id,
        "workspace_id": dest_ws.id,
        "workspace_name": dest_ws.name,
        "project_id": new_project.id,
        "project_name": new_project.name,
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_move.rs"]
mod tests;
