//! Locate `workspace` + `project` (+ document) through the registry.
//!
//! A caller names a workspace by id (default: the built-in default workspace) and a project
//! by folder (default: `inbox`). Nothing is created implicitly, and a project of another
//! workspace is never accepted.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::config::paths;
use crate::services::docs::workspace_dir;
use crate::services::spark_data::{
    self, get_doc, get_project, get_project_by_folder, get_workspace, list_workspaces, DocRow,
    ProjectRow, WorkspaceRow,
};

pub const DEFAULT_PROJECT_FOLDER: &str = "inbox";

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

/// `workspace` (an id) or the default workspace when it is absent.
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

/// The project with this folder inside `workspace`; 400 when it does not exist there.
pub(super) fn resolve_project(
    conn: &Connection,
    workspace: &WorkspaceRow,
    folder: &str,
) -> Result<ProjectRow, Value> {
    get_project_by_folder(conn, &workspace.id, folder)
        .map_err(internal)?
        .ok_or_else(|| {
            bad_request(format!("Unknown project: {folder} in workspace {}", workspace.id))
        })
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

pub(super) fn is_valid_doc_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn doc_id_field(payload: &Value) -> Result<String, Value> {
    let Some(id) = text_field(payload, "id") else {
        return Err(bad_request("Missing id"));
    };
    if !is_valid_doc_id(id) {
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
