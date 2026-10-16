//! Workspace CRUD: `list` / `create` / `rename` / `set_view_mode` / `delete`.
//!
//! `create` takes only a display name and an optional view mode; the id and the document
//! table are made by the Host. A new workspace gets an empty `inbox` project so documents
//! can be written to it right away.

use serde_json::{json, Value};

use crate::services::spark_data::{
    create_workspace as create_workspace_row, delete_workspace as delete_workspace_row,
    list_workspaces as list_workspace_rows, rename_workspace as rename_workspace_row,
    set_workspace_view_mode, ViewMode, WorkspaceRow,
};

use super::project::insert_inbox_project;
use super::resolve::{
    bad_request, conflict, internal, not_found, open_data, remove_empty_tree, resolve_workspace,
    text_field, workspace_root,
};

fn workspace_value(row: &WorkspaceRow) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "view_mode": row.view_mode.as_str(),
        "is_default": row.is_default,
        "created_at": row.created_at,
    })
}

/// Registry errors are plain strings; give the caller a status that fits the cause.
fn registry_error(message: String) -> Value {
    if message.contains("must not be empty") || message.contains("cannot be deleted") {
        bad_request(message)
    } else if message.contains("UNIQUE") {
        conflict("Duplicate workspace name")
    } else if message.contains("still has") {
        conflict(message)
    } else {
        internal(message)
    }
}

fn parse_view_mode(payload: &Value) -> Result<Option<ViewMode>, Value> {
    match text_field(payload, "view_mode") {
        None => Ok(None),
        Some(raw) => ViewMode::parse(raw).map(Some).map_err(bad_request),
    }
}

pub fn list_workspaces() -> Value {
    let rows = open_data().and_then(|conn| list_workspace_rows(&conn).map_err(internal));
    match rows {
        Ok(rows) => json!({ "workspaces": rows.iter().map(workspace_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ name, view_mode? }` → `{ ok, workspace }`.
pub fn create_workspace(payload: &Value) -> Value {
    let Some(name) = text_field(payload, "name") else {
        return bad_request("Missing name");
    };
    let mode = match parse_view_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let row = match create_workspace_row(&conn, name, mode) {
        Ok(row) => row,
        Err(e) => return registry_error(e),
    };
    if let Err(e) = insert_inbox_project(&conn, &row.id) {
        let _ = delete_workspace_row(&conn, &row.id);
        return internal(e);
    }
    if let Err(v) = workspace_root(&row) {
        let _ = delete_workspace_row(&conn, &row.id);
        return v;
    }
    json!({ "ok": true, "workspace": workspace_value(&row) })
}

/// `{ id, name }` → `{ ok, workspace }`. The id never changes.
pub fn rename_workspace(payload: &Value) -> Value {
    let (Some(id), Some(name)) = (text_field(payload, "id"), text_field(payload, "name")) else {
        return bad_request("Missing id or name");
    };
    update_workspace(id, |conn| rename_workspace_row(conn, id, name))
}

/// `{ id, view_mode }` → `{ ok, workspace }`.
pub fn set_view_mode(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    let mode = match parse_view_mode(payload) {
        Ok(Some(m)) => m,
        Ok(None) => return bad_request("Missing view_mode"),
        Err(v) => return v,
    };
    update_workspace(id, |conn| set_workspace_view_mode(conn, id, mode))
}

fn update_workspace(
    id: &str,
    apply: impl FnOnce(&rusqlite::Connection) -> Result<bool, String>,
) -> Value {
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    match apply(&conn) {
        Ok(true) => match resolve_workspace(&conn, Some(id)) {
            Ok(row) => json!({ "ok": true, "workspace": workspace_value(&row) }),
            Err(v) => v,
        },
        Ok(false) => not_found(format!("Unknown workspace: {id}")),
        Err(e) => registry_error(e),
    }
}

/// `{ id }`. The default workspace and a workspace that still holds documents are refused.
pub fn delete_workspace(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let row = match resolve_workspace(&conn, Some(id)) {
        Ok(row) => row,
        Err(v) => return v,
    };
    match delete_workspace_row(&conn, id) {
        Ok(true) => {
            if let Ok(dir) = workspace_root(&row) {
                remove_empty_tree(&dir);
            }
            json!({ "ok": true })
        }
        Ok(false) => not_found(format!("Unknown workspace: {id}")),
        Err(e) => registry_error(e),
    }
}
