//! `move_folder`: move a folder and its whole subtree under another folder of the same
//! project (the project root is the empty target).
//!
//! The copy model from #113: every document is staged, committed, and cleaned up on its
//! own, so an interruption leaves each document whole and at most a split folder behind.
//! Folder rows migrate only after every document moved.

use std::collections::HashSet;
use std::path::Path;

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::archive_parse::is_valid_common_path;
use crate::services::search_index;
use crate::services::spark_data::{list_docs, update_doc as update_doc_row, DocRow};

use super::folder::{folder_known, folder_path_ok, folder_row, insert_folder, project_raw};
use super::folder_rename::{docs_under, folder_paths, retarget_folders, shift_prefix};
use super::folder_rename_disk::{remove_empty, shift_tree, undo_files};
use super::relocate::{cleanup_relocate, plan_relocate_into, stage_relocate};
use super::resolve::{
    bad_request, conflict, ensure_workspace_enabled, internal, not_found, open_data, resolve_project,
    resolve_workspace, text_field, workspace_root,
};
use super::store::doc_file_path;

/// `{ workspace_id, project_id, path, target }` →
/// `{ ok, workspace_id, project_id, path, moved }`.
/// `target` is the destination folder relative to the project; omitted or empty moves
/// the folder to the project root. The folder lands at `<target>/<name>`.
pub fn move_folder(payload: &Value) -> Value {
    let Some(workspace_id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let Some(path) = text_field(payload, "path") else {
        return bad_request("Missing folder");
    };
    if !folder_path_ok(path) {
        return bad_request("Invalid folder");
    }
    let target = text_field(payload, "target").unwrap_or_default();
    if !target.is_empty() && !folder_path_ok(&target) {
        return bad_request("Invalid folder");
    }

    let conn = match open_data() {
        Ok(conn) => conn,
        Err(err) => return err,
    };
    let workspace = match resolve_workspace(&conn, Some(workspace_id)) {
        Ok(workspace) => workspace,
        Err(err) => return err,
    };
    let project = match resolve_project(&conn, &workspace, Some(project_id)) {
        Ok(project) => project,
        Err(err) => return err,
    };
    if let Err(err) = ensure_workspace_enabled(&workspace) {
        return err;
    }
    if !target.is_empty() {
        match folder_known(&conn, &project.id, &target) {
            Ok(true) => {}
            Ok(false) => return bad_request("Unknown folder"),
            Err(err) => return internal(err),
        }
    }

    let new_root = join_dir(&target, basename(path));
    // Dragging a folder onto itself or onto its parent changes nothing.
    if new_root == path || target == path {
        return json!({
            "ok": true,
            "workspace_id": workspace.id,
            "project_id": project.id,
            "path": path,
            "moved": 0,
        });
    }
    // A folder never moves into its own subtree.
    if target.starts_with(&format!("{path}/")) {
        return bad_request("Invalid target");
    }

    let root = match workspace_root(&workspace) {
        Ok(root) => root,
        Err(err) => return err,
    };
    let raw = match project_raw(&root, &project.id) {
        Ok(raw) => raw,
        Err(err) => return err,
    };
    let docs = match docs_under(&conn, &workspace.id, &project.id, path) {
        Ok(docs) => docs,
        Err(err) => return internal(err),
    };
    let rows = match folder_paths(&conn, &project.id, path) {
        Ok(rows) => rows,
        Err(err) => return internal(err),
    };
    let old_dir = raw.join(path);
    if docs.is_empty() && rows.is_empty() && !old_dir.exists() {
        return not_found("Unknown folder");
    }
    let dest_dir = raw.join(&new_root);
    if dest_dir.exists() {
        return conflict("Folder already exists");
    }
    let all_docs = match list_docs(&conn, &workspace.id) {
        Ok(docs) => docs,
        Err(err) => return internal(err),
    };
    let mut moves: Vec<(DocRow, String)> = Vec::with_capacity(docs.len());
    let mut claimed: HashSet<String> = HashSet::new();
    for doc in &docs {
        let Some(new_raw) = shift_prefix(&doc.raw_path, path, &new_root) else {
            continue;
        };
        if !is_valid_common_path(&new_raw) {
            return bad_request("Invalid folder");
        }
        let taken = all_docs
            .iter()
            .any(|d| d.project_id == project.id && d.doc_id != doc.doc_id && d.raw_path == new_raw)
            || doc_file_path(&root, "raw", &project.id, &new_raw)
                .map(|p| p.exists())
                .unwrap_or(false);
        if taken {
            return conflict("File already exists");
        }
        claimed.insert(new_raw.clone());
        moves.push((doc.clone(), new_raw));
    }

    let total = moves.len();
    let mut moved = 0usize;
    for (doc, new_raw) in &moves {
        if let Err(v) = commit_one(&conn, &root, doc, &project.id, new_raw) {
            return stopped(moved, total, &doc.doc_id, v);
        }
        moved += 1;
    }

    // Folder rows migrate only after every document moved.
    if let Err(err) = retarget_folders(&conn, &project.id, path, &new_root) {
        return internal(err);
    }
    if !matches!(folder_row(&conn, &project.id, &new_root), Ok(true)) {
        let _ = insert_folder(&conn, &project.id, &new_root);
    }
    // Files Spark does not track follow the subtree too; empty leftovers are dropped.
    if old_dir.exists() {
        let mut leftover = Vec::new();
        if let Err(message) = shift_tree(&old_dir, &dest_dir, &mut leftover) {
            undo_files(&leftover);
            return internal(message);
        }
    }
    remove_empty(&old_dir);
    if let Ok(digest_dir) = doc_file_path(&root, "digest", &project.id, path) {
        remove_empty(&digest_dir);
    }

    let ids: Vec<&str> = moves.iter().map(|(doc, _)| doc.doc_id.as_str()).collect();
    if !ids.is_empty() {
        search_index::sync_workspace_docs_best_effort(&workspace.id, &ids);
    }
    json!({
        "ok": true,
        "workspace_id": workspace.id,
        "project_id": project.id,
        "path": new_root,
        "moved": moved,
    })
}

/// Copy → commit → cleanup for one document. The source keeps its files until the
/// database points at the destination, so an interruption never breaks the document.
fn commit_one(conn: &Connection, root: &Path, doc: &DocRow, project_id: &str, new_raw: &str) -> Result<(), Value> {
    let plan = plan_relocate_into(doc, project_id, new_raw).map_err(internal)?;
    stage_relocate(root, root, &plan)?;
    let mut next = doc.clone();
    next.raw_path = plan.target.raw_path.clone();
    if next.digest_path.is_some() {
        next.digest_path = plan.target.digest_path.clone();
    }
    match update_doc_row(conn, &next) {
        Ok(true) => {}
        Ok(false) => return Err(internal("document missing")),
        Err(e) => return Err(internal(e)),
    }
    cleanup_relocate(root, &plan)
}

fn stopped(moved: usize, total: usize, failed_id: &str, cause: Value) -> Value {
    let detail = cause.get("error").and_then(Value::as_str).unwrap_or("move failed");
    json!({
        "error": format!("Moved {moved} of {total}; stopped at {failed_id}: {detail}"),
        "_status": 500,
        "moved": moved,
        "total": total,
        "failed_id": failed_id,
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
#[path = "../../unit-tests/services/docs_move_folder.rs"]
mod tests;
