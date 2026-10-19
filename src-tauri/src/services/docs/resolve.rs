//! Locate `workspace` + `project` (+ document) through the registry.
//!
//! A caller names a workspace by id (default: the built-in default workspace) and a project
//! by id (default: the workspace inbox). Nothing is created implicitly, and a project of
//! another workspace is never accepted.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::config::paths;
use crate::services::docs::workspace_dir;
use crate::services::spark_data::{
    self, get_doc, get_inbox_project, get_project, get_workspace, list_workspaces, DocRow,
    ProjectRow, WorkspaceRow,
};

pub(super) fn bad_request(message: impl Into<String>) -> Value {
    json!({ "error": message.into(), "_status": 400 })
}

pub(super) fn not_found(message: impl Into<String>) -> Value {
    json!({ "error": message.into(), "_status": 404 })
}

pub(super) fn conflict(message: impl Into<String>) -> Value {
    json!({ "error": message.into(), "_status": 409 })
}

pub(super) fn internal(message: impl Into<String>) -> Value {
    json!({ "error": message.into(), "_status": 500 })
}

pub(super) fn open_data() -> Result<Connection, Value> {
    spark_data::open().map_err(internal)
}

/// Optional non-empty string field of the payload.
pub(super) fn text_field<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Whether `value` is shaped like a built-in icon id: a two-digit category `00`-`06` and four
/// lowercase letters or digits. Whether that icon exists is the frontend's business.
pub(super) fn is_icon_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 6
        && bytes[0] == b'0'
        && (b'0'..=b'6').contains(&bytes[1])
        && bytes[2..].iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// The `icon` field of a payload: `None` when absent (leave as it is), `Some(None)` for an empty
/// string (clear it), `Some(Some(id))` for a well-formed id. Anything else is a 400.
pub(super) fn icon_field(payload: &Value) -> Result<Option<Option<String>>, Value> {
    match payload.get("icon") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => {
            let icon = raw.trim();
            if icon.is_empty() {
                Ok(Some(None))
            } else if is_icon_id(icon) {
                Ok(Some(Some(icon.to_string())))
            } else {
                Err(bad_request("Invalid icon"))
            }
        }
        Some(_) => Err(bad_request("Invalid icon")),
    }
}

/// A required array of non-empty strings (`ids`, `folders`, ...). Missing or malformed is a 400.
pub(super) fn string_list_field(payload: &Value, key: &str) -> Result<Vec<String>, Value> {
    let Some(Value::Array(items)) = payload.get(key) else {
        return Err(bad_request(format!("Missing {key}")));
    };
    items
        .iter()
        .map(|item| match item {
            Value::String(s) if !s.trim().is_empty() => Ok(s.trim().to_string()),
            _ => Err(bad_request(format!("Invalid {key}"))),
        })
        .collect()
}

/// `workspace_id` or the default workspace when it is absent.
pub(super) fn resolve_workspace(conn: &Connection, requested: Option<&str>) -> Result<WorkspaceRow, Value> {
    match requested {
        Some(id) => get_workspace(conn, id)
            .map_err(internal)?
            .ok_or_else(|| not_found(format!("Unknown workspace: {id}"))),
        None => list_workspaces(conn)
            .map_err(internal)?
            .into_iter()
            .find(|w| w.is_default)
            .ok_or_else(|| internal("default workspace missing")),
    }
}

/// The workspace of a request about document `id`: the `workspace_id` field when given, else the
/// one a prefixed id names, else the default workspace.
pub(super) fn resolve_doc_workspace(
    conn: &Connection,
    payload: &Value,
    id: &str,
) -> Result<WorkspaceRow, Value> {
    if let Some(id) = text_field(payload, "workspace_id") {
        return resolve_workspace(conn, Some(id));
    }
    resolve_workspace_from_doc_id(conn, id)
}

/// The workspace a prefixed document id names; the default workspace when the id has no prefix.
pub(super) fn resolve_workspace_from_doc_id(
    conn: &Connection,
    id: &str,
) -> Result<WorkspaceRow, Value> {
    let named = spark_data::split_doc_id(id).and_then(|(workspace, _)| workspace);
    resolve_workspace(conn, named)
}

/// The project with this id inside `workspace`. Omitted id → the inbox. 400 when missing
/// or when the id belongs to another workspace.
pub(super) fn resolve_project(
    conn: &Connection,
    workspace: &WorkspaceRow,
    id: Option<&str>,
) -> Result<ProjectRow, Value> {
    let row = match id {
        Some(id) => get_project(conn, id)
            .map_err(internal)?
            .ok_or_else(|| bad_request(format!("Unknown project: {id}")))?,
        None => get_inbox_project(conn, &workspace.id)
            .map_err(internal)?
            .ok_or_else(|| bad_request(format!("Inbox missing in workspace {}", workspace.id)))?,
    };
    if row.workspace_id != workspace.id {
        return Err(bad_request(format!(
            "Unknown project: {} in workspace {}",
            row.id, workspace.id
        )));
    }
    Ok(row)
}

/// 409 when `workspace` is disabled. Guards every write; reads by id keep working.
pub(super) fn ensure_workspace_enabled(workspace: &WorkspaceRow) -> Result<(), Value> {
    if workspace.disabled_at.is_some() {
        return Err(conflict(format!("Workspace {} is disabled", workspace.id)));
    }
    Ok(())
}

/// A document of `workspace` plus the project it belongs to.
pub(super) fn resolve_doc(
    conn: &Connection,
    workspace: &WorkspaceRow,
    id: &str,
) -> Result<(DocRow, ProjectRow), Value> {
    let doc = get_doc(conn, &workspace.doc_table, id)
        .map_err(internal)?
        .ok_or_else(|| not_found("Entry not found"))?;
    let project = get_project(conn, &doc.project_id)
        .map_err(internal)?
        .ok_or_else(|| internal(format!("project {} missing", doc.project_id)))?;
    Ok((doc, project))
}

pub(super) fn doc_id_field(payload: &Value) -> Result<String, Value> {
    let Some(id) = text_field(payload, "id") else {
        return Err(bad_request("Missing id"));
    };
    if !spark_data::is_valid_doc_id(id) {
        return Err(bad_request("Invalid id"));
    }
    Ok(id.to_string())
}

/// Remove `dir` and its subfolders, but only if no file is left in them.
pub(super) fn remove_empty_tree(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut empty = true;
    for entry in entries.flatten() {
        let path = entry.path();
        if !(path.is_dir() && !path.is_symlink() && remove_empty_tree(&path)) {
            empty = false;
        }
    }
    empty && std::fs::remove_dir(dir).is_ok()
}

/// The workspace folder on disk, created when missing.
pub(crate) fn workspace_root(workspace: &WorkspaceRow) -> Result<PathBuf, Value> {
    let root = paths::workspaces_root().map_err(|e| internal(format!("{e:?}")))?;
    let dir = workspace_dir(&root, &workspace.id)?;
    std::fs::create_dir_all(&dir).map_err(|e| internal(e.to_string()))?;
    Ok(dir)
}
