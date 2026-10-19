//! Project operations inside one workspace: `list` / `create` / `update` / `reorder`.
//!
//! A project is addressed by its id. The inbox is the `is_inbox` row; its name can change.

use std::collections::HashMap;

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::spark_data::{
    count_docs_by_project, count_docs_in_project, insert_project, mint_project_id,
    list_projects as list_project_rows, reorder_projects as reorder_project_rows,
    update_project as update_project_row, ProjectRow, WorkspaceRow,
};

use super::resolve::{
    bad_request, ensure_workspace_enabled, icon_field, internal,
    open_data, resolve_project, resolve_workspace, string_list_field, text_field,
};

/// A project's flat fields with `ok: true` in front.
fn ok_project(row: &ProjectRow, doc_count: i64) -> Value {
    let mut value = project_value(row, doc_count);
    value["ok"] = json!(true);
    value
}

fn project_value(row: &ProjectRow, doc_count: i64) -> Value {
    json!({
        "project_id": row.id,
        "project_name": row.name,
        "description": row.description,
        "icon": row.icon.clone().unwrap_or_default(),
        "sort_index": row.sort_index,
        "is_inbox": row.is_inbox,
        "doc_count": doc_count,
    })
}

/// Every project of the workspace in the user's order, each with its document count.
fn list_project_values(conn: &Connection, workspace: &WorkspaceRow) -> Result<Vec<Value>, Value> {
    let rows = list_project_rows(conn, &workspace.id).map_err(internal)?;
    let counts: HashMap<String, i64> = count_docs_by_project(conn, &workspace.doc_table).map_err(internal)?;
    Ok(rows.iter().map(|r| project_value(r, counts.get(&r.id).copied().unwrap_or(0))).collect())
}

/// `{ workspace_id? }` → `{ projects: [...] }`.
pub fn list_projects(payload: &Value) -> Value {
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        list_project_values(&conn, &ws)
    });
    match result {
        Ok(projects) => json!({ "projects": projects }),
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
        Ok(row) => ok_project(&row, 0),
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
        row.name = name.to_string();
        row.description = description.to_string();
        if let Some(icon) = icon {
            row.icon = icon;
        }
        update_project_row(&conn, &row).map_err(internal)?;
        let count = count_docs_in_project(&conn, &ws.doc_table, &row.id).map_err(internal)?;
        Ok((row, count))
    });
    match result {
        Ok((row, count)) => ok_project(&row, count),
        Err(v) => v,
    }
}

/// `{ workspace_id?, ids }` → `{ ok, projects }`. `ids` is every project of the workspace in the
/// new order; a list that does not match is a 400 and nothing changes. `projects` is the list in
/// the new order.
pub fn reorder_projects(payload: &Value) -> Value {
    let ids = match string_list_field(payload, "ids") {
        Ok(ids) => ids,
        Err(v) => return v,
    };
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace_id"))?;
        ensure_workspace_enabled(&ws)?;
        reorder_project_rows(&conn, &ws.id, &ids).map_err(bad_request)?;
        list_project_values(&conn, &ws)
    });
    match result {
        Ok(projects) => json!({ "ok": true, "projects": projects }),
        Err(v) => v,
    }
}
