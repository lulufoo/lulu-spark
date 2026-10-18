//! Project operations inside one workspace: `list` / `create` / `update` / `reorder` / `disable` /
//! `enable`.
//!
//! A project is never deleted. Disabling hides it (and its documents) from lists, pickers and
//! search; the documents stay readable by id and `enable` brings the project back. The id
//! stays reserved while it is disabled.
//!
//! A project is addressed by its id. The inbox is the `is_inbox` row; its name can change.

use rusqlite::Connection;
use serde_json::{json, Value};

use chrono::{FixedOffset, Utc};

use crate::services::id::random_alnum6;
use crate::services::spark_data::{
    disable_project as disable_project_row, enable_project as enable_project_row, get_project,
    insert_project, list_disabled_projects as list_disabled_project_rows,
    list_projects as list_project_rows, reorder_projects as reorder_project_rows,
    update_project as update_project_row, ProjectRow,
};

use super::resolve::{
    bad_request, ensure_project_enabled, ensure_workspace_enabled, icon_field, internal,
    open_data, resolve_project, resolve_workspace, string_list_field, text_field,
};

/// A project's flat fields with `ok: true` in front.
fn ok_project(row: &ProjectRow) -> Value {
    let mut value = project_value(row);
    value["ok"] = json!(true);
    value
}

fn project_value(row: &ProjectRow) -> Value {
    json!({
        "project_id": row.id,
        "project_name": row.name,
        "description": row.description,
        "disabled_at": row.disabled_at,
        "icon": row.icon.clone().unwrap_or_default(),
        "sort_index": row.sort_index,
        "is_inbox": row.is_inbox,
    })
}

/// `YYYYMMDDHHMMSSmmm` (UTC+8) plus six random characters.
fn generate_project_id() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    let now = Utc::now().with_timezone(&tz);
    format!(
        "{}{:03}{}",
        now.format("%Y%m%d%H%M%S"),
        now.timestamp_subsec_millis(),
        random_alnum6()
    )
}

fn mint_project_id(conn: &Connection) -> Result<String, String> {
    for _ in 0..8 {
        let id = generate_project_id();
        if get_project(conn, &id)?.is_none() {
            return Ok(id);
        }
    }
    Err("Could not allocate project id".to_string())
}

/// The empty `inbox` project every workspace starts with.
pub(super) fn insert_inbox_project(conn: &Connection, workspace_id: &str) -> Result<(), String> {
    let id = mint_project_id(conn)?;
    let mut row = ProjectRow::new(&id, workspace_id, "Inbox", "");
    row.is_inbox = true;
    insert_project(conn, &row)
}

/// `{ workspace_id? }` → `{ projects: [...] }`.
pub fn list_projects(payload: &Value) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        list_project_rows(&conn, &ws.id).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "projects": rows.iter().map(project_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ workspace_id? }` → `{ projects: [...] }` of the disabled projects, for the Settings
/// "Disabled" section.
pub fn list_disabled_projects(payload: &Value) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        list_disabled_project_rows(&conn, &ws.id).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "projects": rows.iter().map(project_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ workspace_id?, project_name, description?, icon? }` → `{ ok, project_id, project_name, ... }`.
pub fn create_project(payload: &Value) -> Value {
    let Some(name) = text_field(payload, "project_name") else {
        return bad_request("Missing project_name");
    };
    let description = text_field(payload, "description").unwrap_or("");
    let icon = match icon_field(payload) {
        Ok(icon) => icon.flatten(),
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        ensure_workspace_enabled(&ws)?;
        let id = mint_project_id(&conn).map_err(internal)?;
        let mut row = ProjectRow::new(&id, &ws.id, name, description);
        row.icon = icon;
        insert_project(&conn, &row).map_err(internal)?;
        Ok(row)
    });
    match result {
        Ok(row) => ok_project(&row),
        Err(v) => v,
    }
}

/// `{ workspace_id?, project_id, project_name, description?, icon? }` →
/// `{ ok, project_id, project_name, ... }`. `icon` absent keeps the icon, an empty string clears it.
pub fn update_project(payload: &Value) -> Value {
    let (Some(id), Some(name)) =
        (text_field(payload, "project_id"), text_field(payload, "project_name"))
    else {
        return bad_request("Missing project_id or project_name");
    };
    let description = text_field(payload, "description").unwrap_or("");
    let icon = match icon_field(payload) {
        Ok(icon) => icon,
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        let mut row = resolve_project(&conn, &ws, Some(id))?;
        ensure_project_enabled(&row)?;
        row.name = name.to_string();
        row.description = description.to_string();
        if let Some(icon) = icon {
            row.icon = icon;
        }
        update_project_row(&conn, &row).map_err(internal)?;
        Ok(row)
    });
    match result {
        Ok(row) => ok_project(&row),
        Err(v) => v,
    }
}

/// `{ workspace_id?, ids }` → `{ ok, projects }`. `ids` is every project of the workspace,
/// disabled ones included, in the new order; a list that does not match is a 400 and nothing
/// changes. `projects` is the active list in the new order.
pub fn reorder_projects(payload: &Value) -> Value {
    let ids = match string_list_field(payload, "ids") {
        Ok(ids) => ids,
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        ensure_workspace_enabled(&ws)?;
        reorder_project_rows(&conn, &ws.id, &ids).map_err(bad_request)?;
        list_project_rows(&conn, &ws.id).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "ok": true, "projects": rows.iter().map(project_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ workspace_id?, project_id }` → `{ ok, project_id, project_name, ... }`. The inbox (`is_inbox`) is refused. The
/// documents stay; they are hidden from lists and search and still open by id.
pub fn disable_project(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        let row = resolve_project(&conn, &ws, Some(id))?;
        if row.is_inbox {
            return Err(bad_request("Cannot disable inbox"));
        }
        if row.disabled_at.is_some() {
            ensure_workspace_enabled(&ws)?;
        }
        disable_project_row(&conn, &row.id).map_err(internal)?;
        get_project(&conn, &row.id)
            .map_err(internal)?
            .ok_or_else(|| internal("project missing"))
    });
    match result {
        Ok(row) => ok_project(&row),
        Err(v) => v,
    }
}

/// `{ workspace_id?, project_id }` → `{ ok, project_id, project_name, ... }`. The project and its documents show up again.
/// A disabled workspace has to be enabled first.
pub fn enable_project(payload: &Value) -> Value {
    let Some(id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    toggle_project(payload, id, enable_project_row)
}

fn toggle_project(
    payload: &Value,
    id: &str,
    apply: impl FnOnce(&Connection, &str) -> Result<bool, String>,
) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        let row = resolve_project(&conn, &ws, Some(id))?;
        if row.disabled_at.is_some() {
            ensure_workspace_enabled(&ws)?;
        }
        apply(&conn, &row.id).map_err(internal)?;
        get_project(&conn, &row.id).map_err(internal)?.ok_or_else(|| internal("project missing"))
    });
    match result {
        Ok(row) => ok_project(&row),
        Err(v) => v,
    }
}
