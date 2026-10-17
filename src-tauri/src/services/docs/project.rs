//! Project operations inside one workspace: `list` / `create` / `update` / `reorder` / `disable` /
//! `enable`.
//!
//! A project is never deleted. Disabling hides it (and its documents) from lists, pickers and
//! search; the documents stay readable by id and `enable` brings the project back. Its folder
//! stays taken while it is disabled.
//!
//! A project is addressed by its folder (unique per workspace). The id is Host-made and
//! internal; the folder never changes after creation because documents live under it.

use rusqlite::Connection;
use serde_json::{json, Value};

use chrono::{FixedOffset, Utc};

use crate::services::id::random_alnum6;
use crate::services::spark_data::{
    disable_project as disable_project_row, enable_project as enable_project_row, get_project,
    get_project_by_folder, insert_project, list_disabled_projects as list_disabled_project_rows,
    list_projects as list_project_rows, reorder_projects as reorder_project_rows,
    update_project as update_project_row, ProjectRow,
};

use super::resolve::{
    bad_request, conflict, ensure_project_enabled, ensure_workspace_enabled, icon_field, internal,
    open_data, resolve_project, resolve_workspace, string_list_field, text_field,
    DEFAULT_PROJECT_FOLDER,
};

const FOLDER_MAX_LEN: usize = 64;

fn project_value(row: &ProjectRow) -> Value {
    json!({
        "id": row.id,
        "workspace_id": row.workspace_id,
        "folder": row.folder,
        "title": row.title,
        "description": row.description,
        "disabled_at": row.disabled_at,
        "icon": row.icon.clone().unwrap_or_default(),
        "sort_index": row.sort_index,
        "is_inbox": row.is_inbox,
    })
}

fn is_valid_folder(folder: &str) -> bool {
    !folder.is_empty()
        && folder.len() <= FOLDER_MAX_LEN
        && folder.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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
    let mut row = ProjectRow::new(&id, workspace_id, DEFAULT_PROJECT_FOLDER, "Inbox", "");
    row.is_inbox = true;
    insert_project(conn, &row)
}

/// `{ workspace? }` → `{ projects: [...] }`.
pub fn list_projects(payload: &Value) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        list_project_rows(&conn, &ws.id).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "projects": rows.iter().map(project_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ workspace? }` → `{ projects: [...] }` of the disabled projects, for the Settings
/// "Disabled" section.
pub fn list_disabled_projects(payload: &Value) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        list_disabled_project_rows(&conn, &ws.id).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "projects": rows.iter().map(project_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ workspace?, folder?, title, description?, icon? }` → `{ ok, project }`.
pub fn create_project(payload: &Value) -> Value {
    let Some(title) = text_field(payload, "title") else {
        return bad_request("Missing title");
    };
    let requested = text_field(payload, "folder");
    if requested.is_some_and(|f| !is_valid_folder(f)) {
        return bad_request("Invalid folder");
    }
    let description = text_field(payload, "description").unwrap_or("");
    let icon = match icon_field(payload) {
        Ok(icon) => icon.flatten(),
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        ensure_workspace_enabled(&ws)?;
        let id = mint_project_id(&conn).map_err(internal)?;
        let folder = requested.unwrap_or(&id);
        if let Some(holder) = get_project_by_folder(&conn, &ws.id, folder).map_err(internal)? {
            return Err(conflict(if holder.disabled_at.is_some() {
                format!("Project folder is held by a disabled project: {folder}")
            } else {
                format!("Duplicate project folder: {folder}")
            }));
        }
        let mut row = ProjectRow::new(&id, &ws.id, folder, title, description);
        row.icon = icon;
        insert_project(&conn, &row).map_err(internal)?;
        Ok(row)
    });
    match result {
        Ok(row) => json!({ "ok": true, "project": project_value(&row) }),
        Err(v) => v,
    }
}

/// `{ workspace?, project, title, description?, icon? }` → `{ ok, project }`. Folder is kept.
/// `icon` absent keeps the icon, an empty string clears it.
pub fn update_project(payload: &Value) -> Value {
    let (Some(folder), Some(title)) = (text_field(payload, "project"), text_field(payload, "title"))
    else {
        return bad_request("Missing project or title");
    };
    let description = text_field(payload, "description").unwrap_or("");
    let icon = match icon_field(payload) {
        Ok(icon) => icon,
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        let mut row = resolve_project(&conn, &ws, folder)?;
        ensure_project_enabled(&row)?;
        row.title = title.to_string();
        row.description = description.to_string();
        if let Some(icon) = icon {
            row.icon = icon;
        }
        update_project_row(&conn, &row).map_err(internal)?;
        Ok(row)
    });
    match result {
        Ok(row) => json!({ "ok": true, "project": project_value(&row) }),
        Err(v) => v,
    }
}

/// `{ workspace?, folders }` → `{ ok, projects }`. `folders` is every project of the workspace,
/// disabled ones included, in the new order; a list that does not match is a 400 and nothing
/// changes. `projects` is the active list in the new order.
pub fn reorder_projects(payload: &Value) -> Value {
    let folders = match string_list_field(payload, "folders") {
        Ok(folders) => folders,
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        ensure_workspace_enabled(&ws)?;
        reorder_project_rows(&conn, &ws.id, &folders).map_err(bad_request)?;
        list_project_rows(&conn, &ws.id).map_err(internal)
    });
    match result {
        Ok(rows) => json!({ "ok": true, "projects": rows.iter().map(project_value).collect::<Vec<_>>() }),
        Err(v) => v,
    }
}

/// `{ workspace?, project }` → `{ ok, project }`. The inbox (`is_inbox`) is refused. The
/// documents stay; they are hidden from lists and search and still open by id.
pub fn disable_project(payload: &Value) -> Value {
    let Some(folder) = text_field(payload, "project") else {
        return bad_request("Missing project");
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        let row = resolve_project(&conn, &ws, folder)?;
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
        Ok(row) => json!({ "ok": true, "project": project_value(&row) }),
        Err(v) => v,
    }
}

/// `{ workspace?, project }` → `{ ok, project }`. The project and its documents show up again.
/// A disabled workspace has to be enabled first.
pub fn enable_project(payload: &Value) -> Value {
    let Some(folder) = text_field(payload, "project") else {
        return bad_request("Missing project");
    };
    toggle_project(payload, folder, enable_project_row)
}

fn toggle_project(
    payload: &Value,
    folder: &str,
    apply: impl FnOnce(&Connection, &str) -> Result<bool, String>,
) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        let row = resolve_project(&conn, &ws, folder)?;
        if row.disabled_at.is_some() {
            ensure_workspace_enabled(&ws)?;
        }
        apply(&conn, &row.id).map_err(internal)?;
        get_project(&conn, &row.id).map_err(internal)?.ok_or_else(|| internal("project missing"))
    });
    match result {
        Ok(row) => json!({ "ok": true, "project": project_value(&row) }),
        Err(v) => v,
    }
}
