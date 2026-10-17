//! Workspace operations: `list` / `create` / `rename` / `set_view_mode` / `set_icon` / `reorder` /
//! `disable` / `enable`.
//!
//! A workspace is never deleted. Disabling hides it (and its documents) from lists and search;
//! the data stays and `enable` brings it back. Its name stays taken while it is disabled.
//!
//! `create` takes only a display name and an optional view mode; the id and the document
//! table are made by the Host. A new workspace gets an empty `inbox` project so documents
//! can be written to it right away.

use serde_json::{json, Value};

use crate::services::spark_data::{
    create_workspace as create_workspace_row, disable_workspace as disable_workspace_row,
    discard_new_workspace, enable_workspace as enable_workspace_row, get_workspace,
    get_workspace_by_name,
    list_disabled_workspaces as list_disabled_workspace_rows, list_workspaces as list_workspace_rows,
    rename_workspace as rename_workspace_row, reorder_workspaces as reorder_workspace_rows,
    set_workspace_icon as set_workspace_icon_row,
    set_workspace_view_mode, ViewMode, WorkspaceRow,
};

use super::project::insert_inbox_project;
use super::resolve::{
    bad_request, conflict, icon_field, internal, not_found, open_data, resolve_workspace,
    string_list_field, text_field, workspace_root,
};

fn workspace_value(row: &WorkspaceRow) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "view_mode": row.view_mode.as_str(),
        "is_default": row.is_default,
        "created_at": row.created_at,
        "disabled_at": row.disabled_at,
        "icon": row.icon.clone().unwrap_or_default(),
    })
}

/// Registry errors are plain strings; give the caller a status that fits the cause.
fn registry_error(message: String) -> Value {
    if message.contains("must not be empty") || message.contains("cannot be disabled") {
        bad_request(message)
    } else if message.contains("UNIQUE") {
        conflict("Duplicate workspace name")
    } else if message.contains("held by a disabled") || message.contains("Duplicate workspace name") {
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

/// 409 when `name` is already used by another workspace, saying so when that one is disabled.
fn ensure_name_free(conn: &rusqlite::Connection, name: &str, own_id: Option<&str>) -> Result<(), Value> {
    match get_workspace_by_name(conn, name).map_err(internal)? {
        Some(holder) if Some(holder.id.as_str()) != own_id => Err(conflict(if holder.disabled_at.is_some() {
            "Workspace name is held by a disabled workspace".to_string()
        } else {
            "Duplicate workspace name".to_string()
        })),
        _ => Ok(()),
    }
}

pub fn list_workspaces() -> Value {
    let rows = open_data().and_then(|conn| list_workspace_rows(&conn).map_err(internal));
    match rows {
        Ok(rows) => json!({ "workspaces": rows.iter().map(workspace_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// The disabled workspaces, for the Settings "Disabled" section.
pub fn list_disabled_workspaces() -> Value {
    let rows = open_data().and_then(|conn| list_disabled_workspace_rows(&conn).map_err(internal));
    match rows {
        Ok(rows) => json!({ "workspaces": rows.iter().map(workspace_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ name, view_mode?, icon? }` → `{ ok, workspace }`.
pub fn create_workspace(payload: &Value) -> Value {
    let Some(name) = text_field(payload, "name") else {
        return bad_request("Missing name");
    };
    let mode = match parse_view_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let icon = match icon_field(payload) {
        Ok(icon) => icon.flatten(),
        Err(v) => return v,
    };
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    if let Err(v) = ensure_name_free(&conn, name, None) {
        return v;
    }
    let row = match create_workspace_row(&conn, name, mode) {
        Ok(row) => row,
        Err(e) => return registry_error(e),
    };
    if let Err(e) = insert_inbox_project(&conn, &row.id) {
        let _ = discard_new_workspace(&conn, &row.id);
        return internal(e);
    }
    if let Err(v) = workspace_root(&row) {
        let _ = discard_new_workspace(&conn, &row.id);
        return v;
    }
    let row = match icon {
        None => row,
        Some(icon) => {
            let saved = set_workspace_icon_row(&conn, &row.id, Some(&icon))
                .and_then(|_| get_workspace(&conn, &row.id).map(|r| r.unwrap_or(row.clone())));
            match saved {
                Ok(updated) => updated,
                Err(e) => {
                    let _ = discard_new_workspace(&conn, &row.id);
                    return internal(e);
                }
            }
        }
    };
    json!({ "ok": true, "workspace": workspace_value(&row) })
}

/// `{ id, name }` → `{ ok, workspace }`. The id never changes.
pub fn rename_workspace(payload: &Value) -> Value {
    let (Some(id), Some(name)) = (text_field(payload, "id"), text_field(payload, "name")) else {
        return bad_request("Missing id or name");
    };
    update_workspace(id, |conn| {
        ensure_name_free(conn, name, Some(id)).map_err(|v| v["error"].as_str().unwrap_or("").to_string())?;
        rename_workspace_row(conn, id, name)
    })
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

/// `{ id, icon }` → `{ ok, workspace }`. An empty `icon` clears it (back to the default icon).
pub fn set_icon(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    let icon = match icon_field(payload) {
        Ok(Some(icon)) => icon,
        Ok(None) => return bad_request("Missing icon"),
        Err(v) => return v,
    };
    update_workspace(id, |conn| set_workspace_icon_row(conn, id, icon.as_deref()))
}

/// `{ ids }` → `{ ok, workspaces }`. `ids` is every workspace, disabled ones included, in the
/// new order; a list that does not match is a 400 and nothing changes. `workspaces` is the
/// active list in the new order.
pub fn reorder_workspaces(payload: &Value) -> Value {
    let ids = match string_list_field(payload, "ids") {
        Ok(ids) => ids,
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        reorder_workspace_rows(&conn, &ids).map_err(bad_request)?;
        list_workspace_rows(&conn).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "ok": true, "workspaces": rows.iter().map(workspace_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
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

/// `{ id }` → `{ ok, workspace }`. The default workspace is refused. Documents stay; they are
/// hidden from lists and search and still open by id.
pub fn disable_workspace(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    update_workspace(id, |conn| disable_workspace_row(conn, id))
}

/// `{ id }` → `{ ok, workspace }`. The workspace and its documents show up again.
pub fn enable_workspace(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    update_workspace(id, |conn| enable_workspace_row(conn, id))
}
