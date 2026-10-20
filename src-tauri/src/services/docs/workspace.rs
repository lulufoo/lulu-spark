//! Workspace operations: `list` / `create` / `rename` / `set_view_mode` / `set_icon` / `set_description` / `reorder` /
//! `disable` / `enable`.
//!
//! A workspace is never deleted. Disabling hides it (and its documents) from lists and search;
//! the data stays and `enable` brings it back. Its name stays taken while it is disabled.
//!
//! `create` takes only a display name and an optional view mode; the id and the document
//! table are made by the Host. A new workspace gets an empty inbox project (made with the workspace
//! row, in one transaction) so documents can be written to it right away.

use serde_json::{json, Value};

use crate::services::spark_data::{
    create_workspace as create_workspace_row, disable_workspace as disable_workspace_row,
    discard_new_workspace, enable_workspace as enable_workspace_row, get_workspace,
    get_workspace_by_name,
    list_disabled_workspaces as list_disabled_workspace_rows, list_workspaces as list_workspace_rows,
    rename_workspace as rename_workspace_row, reorder_workspaces as reorder_workspace_rows,
    set_workspace_description as set_workspace_description_row,
    set_workspace_icon as set_workspace_icon_row,
    set_workspace_view_mode, ViewMode, WorkspaceRow,
};

use super::resolve::{
    bad_request, conflict, icon_field, internal, not_found, open_data, resolve_workspace,
    string_list_field, text_field, workspace_root,
};

fn workspace_value(row: &WorkspaceRow) -> Value {
    json!({
        "workspace_id": row.id,
        "workspace_name": row.name,
        "view_mode": row.view_mode.as_str(),
        "is_default": row.is_default,
        "created_at": row.created_at,
        "disabled_at": row.disabled_at,
        "icon": row.icon.clone().unwrap_or_default(),
        "sort_index": row.sort_index,
        "description": row.description,
    })
}

/// `None` when absent; `Some(trimmed)` when a string, including empty.
fn description_field(payload: &Value) -> Result<Option<String>, Value> {
    match payload.get("description") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => Ok(Some(raw.trim().to_string())),
        Some(_) => Err(bad_request("Invalid description")),
    }
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

/// `{ workspace_name, view_mode?, icon?, description? }` → `{ ok, workspace_id, workspace_name, ... }`.
pub fn create_workspace(payload: &Value) -> Value {
    let Some(name) = text_field(payload, "workspace_name") else {
        return bad_request("Missing workspace_name");
    };
    let mode = match parse_view_mode(payload) {
        Ok(m) => m,
        Err(v) => return v,
    };
    let icon = match icon_field(payload) {
        Ok(icon) => icon.flatten(),
        Err(v) => return v,
    };
    let description = match description_field(payload) {
        Ok(description) => description,
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
    if let Err(v) = workspace_root(&row) {
        let _ = discard_new_workspace(&conn, &row.id);
        return v;
    }
    let row = match apply_create_extras(&conn, row, icon.as_deref(), description.as_deref()) {
        Ok(row) => row,
        Err(v) => return v,
    };
    ok_workspace(&row)
}

fn apply_create_extras(
    conn: &rusqlite::Connection,
    row: WorkspaceRow,
    icon: Option<&str>,
    description: Option<&str>,
) -> Result<WorkspaceRow, Value> {
    if let Some(icon) = icon {
        set_workspace_icon_row(conn, &row.id, Some(icon)).map_err(|e| {
            let _ = discard_new_workspace(conn, &row.id);
            internal(e)
        })?;
    }
    if let Some(description) = description {
        set_workspace_description_row(conn, &row.id, description).map_err(|e| {
            let _ = discard_new_workspace(conn, &row.id);
            internal(e)
        })?;
    }
    if icon.is_none() && description.is_none() {
        return Ok(row);
    }
    get_workspace(conn, &row.id)
        .map_err(|e| {
            let _ = discard_new_workspace(conn, &row.id);
            internal(e)
        })?
        .ok_or_else(|| {
            let _ = discard_new_workspace(conn, &row.id);
            internal("workspace row missing after create extras".to_string())
        })
}

/// A workspace's flat fields with `ok: true` in front.
fn ok_workspace(row: &WorkspaceRow) -> Value {
    let mut value = workspace_value(row);
    value["ok"] = json!(true);
    value
}

/// `{ workspace_id, workspace_name }` → `{ ok, workspace_id, workspace_name, ... }`. The id never changes.
pub fn rename_workspace(payload: &Value) -> Value {
    let (Some(id), Some(name)) =
        (text_field(payload, "workspace_id"), text_field(payload, "workspace_name"))
    else {
        return bad_request("Missing workspace_id or workspace_name");
    };
    update_workspace(id, |conn| {
        ensure_name_free(conn, name, Some(id)).map_err(|v| v["error"].as_str().unwrap_or("").to_string())?;
        rename_workspace_row(conn, id, name)
    })
}

/// `{ workspace_id, view_mode }` → `{ ok, workspace_id, workspace_name, ... }`.
pub fn set_view_mode(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let mode = match parse_view_mode(payload) {
        Ok(Some(m)) => m,
        Ok(None) => return bad_request("Missing view_mode"),
        Err(v) => return v,
    };
    update_workspace(id, |conn| set_workspace_view_mode(conn, id, mode))
}

/// `{ workspace_id, icon }` → `{ ok, workspace_id, workspace_name, ... }`. An empty `icon` clears it
/// (back to the default icon).
pub fn set_icon(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let icon = match icon_field(payload) {
        Ok(Some(icon)) => icon,
        Ok(None) => return bad_request("Missing icon"),
        Err(v) => return v,
    };
    update_workspace(id, |conn| set_workspace_icon_row(conn, id, icon.as_deref()))
}

/// `{ workspace_id, description }` → `{ ok, workspace_id, workspace_name, ... }`. An empty
/// `description` clears it.
pub fn set_description(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let description = match description_field(payload) {
        Ok(Some(description)) => description,
        Ok(None) => return bad_request("Missing description"),
        Err(v) => return v,
    };
    update_workspace(id, |conn| set_workspace_description_row(conn, id, &description))
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
            Ok(row) => ok_workspace(&row),
            Err(v) => v,
        },
        Ok(false) => not_found(format!("Unknown workspace: {id}")),
        Err(e) => registry_error(e),
    }
}

/// `{ workspace_id }` → `{ ok, workspace_id, workspace_name, ... }`. The default workspace is
/// refused. Documents stay; they are hidden from lists and search and still open by id.
pub fn disable_workspace(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    update_workspace(id, |conn| disable_workspace_row(conn, id))
}

/// `{ workspace_id }` → `{ ok, workspace_id, workspace_name, ... }`. The workspace and its
/// documents show up again.
pub fn enable_workspace(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    update_workspace(id, |conn| enable_workspace_row(conn, id))
}
