//! `delete_workspace`: move every document to the default workspace inbox, then drop
//! the emptied workspace.
//!
//! One `move_doc` per document (stop at the first failure), then one transaction for
//! the projects and the registry row. Empty folders are pruned; leftover files stay.

use std::fs;

use serde_json::{json, Value};

use crate::config::paths;
use crate::services::spark_data::{delete_empty_workspace, list_docs, list_projects};

use super::move_doc::move_doc;
use super::resolve::{
    bad_request, ensure_workspace_enabled, internal, open_data, remove_empty_tree, resolve_project,
    resolve_workspace, text_field, workspace_root,
};

/// `{ workspace_id }` → `{ ok, workspace_id, moved, projects_removed, folder_removed }`.
pub fn delete_workspace(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    delete_named(id).unwrap_or_else(|v| v)
}

fn delete_named(id: &str) -> Result<Value, Value> {
    let conn = open_data()?;
    let workspace = resolve_workspace(&conn, Some(id))?;
    if workspace.is_default {
        return Err(bad_request("Cannot delete the default workspace"));
    }
    ensure_workspace_enabled(&workspace)?;
    let dest = resolve_workspace(&conn, None)?;
    let inbox = resolve_project(&conn, &dest, None)?;
    let docs = list_docs(&conn, &workspace.id).map_err(internal)?;
    let projects_removed = list_projects(&conn, &workspace.id).map_err(internal)?.len();
    let root = workspace_root(&workspace)?;
    drop(conn);

    let total = docs.len();
    let mut moved = 0usize;
    for doc in &docs {
        let result = move_doc(&json!({
            "id": doc.doc_id,
            "project_id": inbox.id,
            "workspace_id": workspace.id,
            "target_workspace_id": dest.id,
        }));
        if result.get("ok") != Some(&json!(true)) {
            return Err(stopped(moved, total, &doc.doc_id, result));
        }
        moved += 1;
    }

    let conn = open_data()?;
    match delete_empty_workspace(&conn, &workspace.id) {
        Ok(true) => {}
        Ok(false) => return Err(internal("workspace missing after move")),
        Err(e) => return Err(leftover_docs(moved, total, e)),
    }
    drop(conn);

    let folder_removed = remove_empty_tree(&root);
    remove_cache(&workspace.id);
    Ok(json!({
        "ok": true,
        "workspace_id": workspace.id,
        "moved": moved,
        "projects_removed": projects_removed,
        "folder_removed": folder_removed,
    }))
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

fn remove_cache(workspace_id: &str) {
    for path in [
        paths::workspace_hide_patterns_path(workspace_id),
        paths::workspace_viewer_state_path(workspace_id),
    ]
    .into_iter()
    .flatten()
    {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_delete_workspace.rs"]
mod tests;
