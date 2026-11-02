//! `move_project`: move every document of a project into another project, then drop the source.
//!
//! One `move_doc` per document (stop at the first failure, or when the user cancels). The
//! document already being copied finishes. The source row goes only after every document has
//! moved. The same workspace and another workspace use this one path.

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::services::spark_data::{delete_empty_project, list_docs};

use super::merge_files::get_project_dir;
use super::move_doc::move_doc;
use super::resolve::{
    bad_request, ensure_workspace_enabled, internal, open_data, remove_empty_tree, resolve_project,
    resolve_workspace, text_field, workspace_root,
};

const LAYERS: &[&str] = &["raw", "digest"];

/// The run that is going on. `0` means none.
static RUN: AtomicU64 = AtomicU64::new(0);
/// Set to [`RUN`] when the user asks the run to stop before the next document.
static CANCELLED_RUN: AtomicU64 = AtomicU64::new(0);

/// Ask the move that is going on to stop before its next document. The one being copied finishes.
pub fn cancel_move_project() -> Value {
    CANCELLED_RUN.store(RUN.load(Ordering::SeqCst), Ordering::SeqCst);
    json!({ "ok": true })
}

fn run_is_cancelled(run: u64) -> bool {
    run != 0 && CANCELLED_RUN.load(Ordering::SeqCst) == run
}

/// `{ workspace_id?, project_id, target_workspace_id, target_project_id }` →
/// `{ ok, project_id, moved, target_workspace_id, target_project_id, target_project_name }`
/// or the same with `cancelled: true` when the user stopped it. The source project stays.
pub fn move_project(payload: &Value) -> Value {
    let run = RUN.fetch_add(1, Ordering::SeqCst) + 1;
    move_named(payload, run).unwrap_or_else(|v| v)
}

fn move_named(payload: &Value, run: u64) -> Result<Value, Value> {
    let Some(project_id) = text_field(payload, "project_id") else {
        return Err(bad_request("Missing project_id"));
    };
    let Some(target_project_id) = text_field(payload, "target_project_id") else {
        return Err(bad_request("Missing target_project_id"));
    };
    let Some(target_workspace_id) = text_field(payload, "target_workspace_id") else {
        return Err(bad_request("Missing target_workspace_id"));
    };

    let conn = open_data()?;
    let source_ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
    let source = resolve_project(&conn, &source_ws, Some(project_id))?;
    if source.is_default {
        return Err(bad_request("Cannot move the built-in project"));
    }
    ensure_workspace_enabled(&source_ws)?;
    let target_ws = resolve_workspace(&conn, Some(target_workspace_id))?;
    ensure_workspace_enabled(&target_ws)?;
    let target = resolve_project(&conn, &target_ws, Some(target_project_id))?;
    if target.id == source.id {
        return Err(bad_request("Already in this project"));
    }
    let docs: Vec<_> = list_docs(&conn, &source_ws.id)
        .map_err(internal)?
        .into_iter()
        .filter(|doc| doc.project_id == source.id)
        .collect();
    let root = workspace_root(&source_ws)?;
    drop(conn);

    let total = docs.len();
    let mut moved = 0usize;
    for doc in &docs {
        if run_is_cancelled(run) {
            return Ok(cancelled(moved, total, &source.id, &target_ws.id, &target.id, &target.name));
        }
        let result = move_doc(&json!({
            "id": doc.doc_id,
            "project_id": target.id,
            "workspace_id": source_ws.id,
            "target_workspace_id": target_ws.id,
        }));
        if result.get("ok") != Some(&json!(true)) {
            return Err(stopped(moved, total, &doc.doc_id, result));
        }
        moved += 1;
    }
    if run_is_cancelled(run) {
        return Ok(cancelled(moved, total, &source.id, &target_ws.id, &target.id, &target.name));
    }

    let conn = open_data()?;
    match delete_empty_project(&conn, &source.id) {
        Ok(true) => {}
        Ok(false) => return Err(internal("project missing after move")),
        Err(message) => return Err(leftover_docs(moved, total, message)),
    }
    drop(conn);

    for layer in LAYERS {
        if let Ok(dir) = get_project_dir(&root, layer, &source.id) {
            remove_empty_tree(&dir);
        }
    }
    Ok(json!({
        "ok": true,
        "project_id": source.id,
        "moved": moved,
        "target_workspace_id": target_ws.id,
        "target_project_id": target.id,
        "target_project_name": target.name,
    }))
}

fn cancelled(moved: usize, total: usize, project_id: &str, workspace_id: &str, target_id: &str, target_name: &str) -> Value {
    json!({
        "ok": true,
        "cancelled": true,
        "moved": moved,
        "total": total,
        "project_id": project_id,
        "target_workspace_id": workspace_id,
        "target_project_id": target_id,
        "target_project_name": target_name,
    })
}

fn leftover_docs(moved: usize, total: usize, message: String) -> Value {
    let status = if message.contains("still has") { 409 } else { 500 };
    json!({
        "error": format!("Moved {moved} of {total} documents; {message}"),
        "_status": status,
        "moved": moved,
        "total": total,
    })
}

fn stopped(moved: usize, total: usize, failed_id: &str, cause: Value) -> Value {
    let detail = cause.get("error").and_then(Value::as_str).unwrap_or("move failed");
    let status = cause.get("_status").cloned().unwrap_or(json!(500));
    json!({
        "error": format!("Moved {moved} of {total} documents; stopped at {failed_id}: {detail}"),
        "_status": status,
        "moved": moved,
        "total": total,
        "failed_id": failed_id,
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_move_project.rs"]
mod tests;
