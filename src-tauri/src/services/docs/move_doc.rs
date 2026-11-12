//! `move_doc`: the one document mover — another folder of the same project, another
//! project, or another workspace.
//!
//! Plan → stage (copy) → database (commit point) → cleanup → index the target workspace
//! once. Copy-first keeps the sources intact until the commit, so any interruption
//! leaves the document whole where the database points and at most orphaned copies
//! behind, which the orphan sweep reclaims.

use serde_json::{json, Value};

use crate::services::archive_parse::is_valid_common_path;
use crate::services::id::random_alnum6;
use crate::services::search_index;
use crate::services::spark_data::{list_docs, update_doc as update_doc_row};

use super::folder::{folder_known, keep_parent_folder};
use super::merge_files::collect_taken_paths;
use super::relocate::{cleanup_relocate, plan_relocate, plan_relocate_into, stage_relocate};
use super::resolve::{
    bad_request, conflict, doc_id_field, ensure_workspace_enabled, internal, open_data, resolve_doc,
    resolve_doc_workspace, resolve_project, resolve_workspace, text_field, workspace_root,
};
use super::store::doc_file_path;

/// `{ id, project_id, workspace_id?, target_workspace_id?, target? }` →
/// `{ ok, id, workspace_id, workspace_name, project_id, project_name, raw_path }`.
/// `workspace_id` must be the document's current workspace when sent.
/// `target_workspace_id` defaults to that workspace. When the destination project is
/// the document's own project, `target` is the destination folder inside it (empty or
/// omitted for the project root), a taken name is a conflict instead of a rename, and
/// the folder the document leaves stays in the tree.
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
    let same_project = new_project.id == old_project.id;
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
    let plan = if same_project {
        let folder = text_field(payload, "target").unwrap_or_default();
        if !folder.is_empty() {
            match folder_known(&conn, &new_project.id, &folder) {
                Ok(true) => {}
                Ok(false) => return bad_request("Unknown folder"),
                Err(err) => return internal(err),
            }
        }
        let new_raw = join_dir(&folder, basename(&rel));
        if !is_valid_common_path(&new_raw) {
            return bad_request("Invalid folder");
        }
        if new_raw == rel {
            return json!({
                "ok": true,
                "id": id,
                "workspace_id": dest_ws.id,
                "workspace_name": dest_ws.name,
                "project_id": new_project.id,
                "project_name": new_project.name,
                "raw_path": rel,
            });
        }
        let taken = dest_docs
            .iter()
            .any(|d| d.doc_id != id && d.raw_path == new_raw)
            || doc_file_path(&dest_root, "raw", &new_project.id, &new_raw)
                .map(|p| p.exists())
                .unwrap_or(false);
        if taken {
            return conflict("File already exists");
        }
        match plan_relocate_into(&doc, &new_project.id, &new_raw) {
            Ok(p) => p,
            Err(e) => return internal(e),
        }
    } else {
        let (taken_raw, taken_digest) = match collect_taken_paths(&dest_root, &new_project.id, &dest_docs)
        {
            Ok(v) => v,
            Err(v) => return v,
        };
        match plan_relocate(
            &doc,
            &new_project.id,
            &taken_raw,
            &taken_digest,
            random_alnum6,
        ) {
            Ok(p) => p,
            Err(e) => return internal(e),
        }
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
    if same_project {
        // The folder the document left stays in the tree when nothing is left inside it.
        if let Err(err) = keep_parent_folder(&conn, &new_project.id, &rel) {
            return internal(err);
        }
    }
    search_index::sync_workspace_docs_best_effort(&dest_ws.id, &[id.as_str()]);
    json!({
        "ok": true,
        "id": id,
        "workspace_id": dest_ws.id,
        "workspace_name": dest_ws.name,
        "project_id": new_project.id,
        "project_name": new_project.name,
        "raw_path": plan.target.raw_path,
    })
}

fn join_dir(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn basename(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_move.rs"]
mod tests;
