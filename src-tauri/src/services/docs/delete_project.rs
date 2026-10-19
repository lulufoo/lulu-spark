//! `delete_project`: give every document of a project to the workspace inbox, then delete the
//! project. All or nothing: the files go back when a move or the database write fails.

use std::path::PathBuf;

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::id::random_alnum6;
use crate::services::search_index;
use crate::services::spark_data::{delete_project_after_merge, list_docs, DocRow, ProjectRow, WorkspaceRow};

use super::merge_files::{collect_taken_paths, get_project_dir, list_doc_moves, run_moves, undo_moves};
use super::merge_plan::{plan_merge_targets, MergeTarget};
use super::resolve::{
    bad_request, ensure_workspace_enabled, internal, open_data, remove_empty_tree, resolve_project,
    resolve_workspace, text_field, workspace_root,
};
use super::store::doc_file_path;

const LAYERS: &[&str] = &["raw", "digest"];

/// `{ workspace_id?, project_id }` →
/// `{ ok, project_id, moved, renamed, target_project_id, target_project_name }`.
/// `moved` counts documents, `renamed` the ones that had to take a new file name.
pub fn delete_project(payload: &Value) -> Value {
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let result = open_data().and_then(|conn| {
        let workspace = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        let project = resolve_project(&conn, &workspace, Some(project_id))?;
        if project.is_inbox {
            return Err(bad_request("Cannot delete the built-in project"));
        }
        ensure_workspace_enabled(&workspace)?;
        let inbox = resolve_project(&conn, &workspace, None)?;
        merge_into_inbox(&conn, &workspace, &project, &inbox)
    });
    result.unwrap_or_else(|v| v)
}

/// The documents of `workspace` that sit in project `project_id`.
fn pick_docs(all: &[DocRow], project_id: &str) -> Vec<DocRow> {
    all.iter().filter(|d| d.project_id == project_id).cloned().collect()
}

/// The doc as the planner should see it: a digest file at `raw_path` with no `digest_path`
/// counts as a digest, so its name is checked too.
fn mark_loose_digest(root: &std::path::Path, project_id: &str, doc: &DocRow) -> Result<DocRow, Value> {
    let mut row = doc.clone();
    if row.digest_path.is_none() && doc_file_path(root, "digest", project_id, &doc.raw_path)?.exists() {
        row.digest_path = Some(doc.raw_path.clone());
    }
    Ok(row)
}

/// The row as it is stored once the doc lives in `inbox` under `target`.
fn move_row(doc: &DocRow, target: &MergeTarget, inbox: &ProjectRow) -> DocRow {
    let mut row = doc.clone();
    row.project_id = inbox.id.clone();
    row.raw_path = target.raw_path.clone();
    // A row that named no digest path keeps naming none.
    row.digest_path = doc.digest_path.as_ref().and(target.digest_path.clone());
    row
}

fn merge_into_inbox(
    conn: &Connection,
    workspace: &WorkspaceRow,
    project: &ProjectRow,
    inbox: &ProjectRow,
) -> Result<Value, Value> {
    let root = workspace_root(workspace)?;
    let all = list_docs(conn, &workspace.id).map_err(internal)?;
    let docs = pick_docs(&all, &project.id);
    let inbox_docs = pick_docs(&all, &inbox.id);

    let planning = docs
        .iter()
        .map(|d| mark_loose_digest(&root, &project.id, d))
        .collect::<Result<Vec<_>, _>>()?;
    let (taken_raw, taken_digest) = collect_taken_paths(&root, &inbox.id, &inbox_docs)?;
    let targets = plan_merge_targets(&planning, &taken_raw, &taken_digest, random_alnum6).map_err(internal)?;

    let mut moves = Vec::new();
    for (doc, target) in docs.iter().zip(&targets) {
        moves.extend(list_doc_moves(&root, &project.id, &inbox.id, doc, target)?);
    }
    let keep: Vec<PathBuf> = LAYERS
        .iter()
        .map(|layer| get_project_dir(&root, layer, &inbox.id))
        .collect::<Result<_, _>>()?;
    let done = run_moves(&moves, &keep).map_err(internal)?;

    let rows: Vec<DocRow> = docs.iter().zip(&targets).map(|(d, t)| move_row(d, t, inbox)).collect();
    if let Err(e) = delete_project_after_merge(conn, &project.id, &rows) {
        undo_moves(&done, &keep);
        return Err(internal(e));
    }
    for layer in LAYERS {
        if let Ok(dir) = get_project_dir(&root, layer, &project.id) {
            remove_empty_tree(&dir);
        }
    }
    let ids: Vec<&str> = docs.iter().map(|d| d.doc_id.as_str()).collect();
    if !ids.is_empty() {
        search_index::sync_workspace_docs_best_effort(&workspace.id, &ids);
    }
    Ok(json!({
        "ok": true,
        "project_id": project.id,
        "moved": docs.len(),
        "renamed": targets.iter().filter(|t| t.renamed).count(),
        "target_project_id": inbox.id,
        "target_project_name": inbox.name,
    }))
}
