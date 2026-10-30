//! `delete_project`: give every document of a project to the workspace inbox, then delete
//! the project.
//!
//! Stage every copy, write rows and drop the project in one transaction, then cleanup
//! sources. Dest copies stay if a later step fails (no rollback).

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::id::random_alnum6;
use crate::services::search_index;
use crate::services::spark_data::{delete_project_after_merge, list_docs, DocRow, ProjectRow, WorkspaceRow};

use super::merge_files::{collect_taken_paths, get_project_dir};
use super::relocate::{cleanup_relocate, plan_relocate_all, stage_relocate, RelocatePlan};
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
        if project.is_default {
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

/// The row as it is stored once the doc lives in `inbox` under `plan`.
fn apply_inbox_target(doc: &DocRow, plan: &RelocatePlan, inbox: &ProjectRow) -> DocRow {
    let mut row = doc.clone();
    row.project_id = inbox.id.clone();
    row.raw_path = plan.target.raw_path.clone();
    if doc.digest_path.is_some() {
        row.digest_path = plan.target.digest_path.clone();
    }
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
    let plans = plan_relocate_all(&planning, &inbox.id, &taken_raw, &taken_digest, random_alnum6)
        .map_err(internal)?;
    for plan in &plans {
        stage_relocate(&root, &root, plan)?;
    }
    let rows: Vec<DocRow> = docs
        .iter()
        .zip(&plans)
        .map(|(doc, plan)| apply_inbox_target(doc, plan, inbox))
        .collect();
    delete_project_after_merge(conn, &project.id, &rows).map_err(internal)?;
    for plan in &plans {
        cleanup_relocate(&root, plan)?;
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
        "renamed": plans.iter().filter(|p| p.target.renamed).count(),
        "target_project_id": inbox.id,
        "target_project_name": inbox.name,
    }))
}
