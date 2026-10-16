//! Project CRUD inside one workspace: `list` / `create` / `update` / `delete`.
//!
//! A project is addressed by its folder (unique per workspace). The id is Host-made and
//! internal; the folder never changes after creation because documents live under it.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::notes::generate_category_id;
use crate::services::spark_data::{
    count_docs_in_project, delete_project as delete_project_row, get_project,
    get_project_by_folder, insert_project,
    list_projects as list_project_rows, update_project as update_project_row, ProjectRow,
};

use super::resolve::{
    bad_request, conflict, internal, open_data, remove_empty_tree, resolve_project,
    resolve_workspace, text_field, workspace_root, DEFAULT_PROJECT_FOLDER,
};

const FOLDER_MAX_LEN: usize = 64;

fn project_value(row: &ProjectRow) -> Value {
    json!({
        "id": row.id,
        "workspace_id": row.workspace_id,
        "folder": row.folder,
        "title": row.title,
        "description": row.description,
    })
}

fn is_valid_folder(folder: &str) -> bool {
    !folder.is_empty()
        && folder.len() <= FOLDER_MAX_LEN
        && folder.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn mint_project_id(conn: &Connection) -> Result<String, String> {
    for _ in 0..8 {
        let id = generate_category_id();
        if get_project(conn, &id)?.is_none() {
            return Ok(id);
        }
    }
    Err("Could not allocate project id".to_string())
}

/// The empty `inbox` project every workspace starts with.
pub(super) fn insert_inbox_project(conn: &Connection, workspace_id: &str) -> Result<(), String> {
    let id = mint_project_id(conn)?;
    insert_project(
        conn,
        &ProjectRow::new(&id, workspace_id, DEFAULT_PROJECT_FOLDER, "Inbox", ""),
    )
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

/// `{ workspace?, folder?, title, description? }` → `{ ok, project }`.
pub fn create_project(payload: &Value) -> Value {
    let Some(title) = text_field(payload, "title") else {
        return bad_request("Missing title");
    };
    let requested = text_field(payload, "folder");
    if requested.is_some_and(|f| !is_valid_folder(f)) {
        return bad_request("Invalid folder");
    }
    let description = text_field(payload, "description").unwrap_or("");
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        let id = mint_project_id(&conn).map_err(internal)?;
        let folder = requested.unwrap_or(&id);
        if get_project_by_folder(&conn, &ws.id, folder).map_err(internal)?.is_some() {
            return Err(conflict(format!("Duplicate project folder: {folder}")));
        }
        let row = ProjectRow::new(&id, &ws.id, folder, title, description);
        insert_project(&conn, &row).map_err(internal)?;
        Ok(row)
    });
    match result {
        Ok(row) => json!({ "ok": true, "project": project_value(&row) }),
        Err(v) => v,
    }
}

/// `{ workspace?, project, title, description? }` → `{ ok, project }`. Folder is kept.
pub fn update_project(payload: &Value) -> Value {
    let (Some(folder), Some(title)) = (text_field(payload, "project"), text_field(payload, "title"))
    else {
        return bad_request("Missing project or title");
    };
    let description = text_field(payload, "description").unwrap_or("");
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        let mut row = resolve_project(&conn, &ws, folder)?;
        row.title = title.to_string();
        row.description = description.to_string();
        update_project_row(&conn, &row).map_err(internal)?;
        Ok(row)
    });
    match result {
        Ok(row) => json!({ "ok": true, "project": project_value(&row) }),
        Err(v) => v,
    }
}

/// `{ workspace?, project }`. `inbox` and a project that still holds documents are refused.
pub fn delete_project(payload: &Value) -> Value {
    let Some(folder) = text_field(payload, "project") else {
        return bad_request("Missing project");
    };
    if folder == DEFAULT_PROJECT_FOLDER {
        return bad_request("Cannot delete inbox");
    }
    let result = open_data().and_then(|conn| {
        let ws = resolve_workspace(&conn, text_field(payload, "workspace"))?;
        let row = resolve_project(&conn, &ws, folder)?;
        let docs = count_docs_in_project(&conn, &ws.doc_table, &row.id).map_err(internal)?;
        if docs > 0 {
            return Err(conflict(format!("Project {folder} still has {docs} document(s)")));
        }
        delete_project_row(&conn, &row.id).map_err(internal)?;
        Ok((ws, row))
    });
    match result {
        Ok((ws, row)) => {
            if let Ok(root) = workspace_root(&ws) {
                for layer in ["raw", "digest"] {
                    remove_empty_tree(&root.join(layer).join(&row.folder));
                }
            }
            json!({ "ok": true })
        }
        Err(v) => v,
    }
}
