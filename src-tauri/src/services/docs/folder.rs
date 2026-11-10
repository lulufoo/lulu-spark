//! An empty directory that stays in the tree when it holds no document.
//!
//! The row and the directory are one create: the new directory is removed if the row
//! cannot be stored. Deleting an empty folder removes that one row and that one directory.
//! A folder that still holds anything is left as it is.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::services::archive_parse::is_valid_common_path;
use crate::services::spark_data::WorkspaceRow;

use super::resolve::{
    bad_request, conflict, ensure_workspace_enabled, internal, not_found, open_data, resolve_project,
    resolve_workspace, text_field, workspace_root,
};

/// `{ workspace_id, project_id, name, parent? }` → `{ ok, project_id, path }`.
/// `parent` is the folder the new one is created under; omit it, or pass `/`, for the project root.
pub fn create_folder(payload: &Value) -> Value {
    let Some(workspace_id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let Some(name) = text_field(payload, "name") else {
        return bad_request("Missing folder name");
    };
    if !one_segment(name) {
        return bad_request("Invalid folder name");
    }
    let parent = match parent_path(payload) {
        Ok(path) => path,
        Err(err) => return err,
    };
    let path = if parent.is_empty() { name.to_string() } else { format!("{parent}/{name}") };
    if !folder_path_ok(&path) {
        return bad_request("Invalid folder");
    }

    let conn = match open_data() {
        Ok(conn) => conn,
        Err(err) => return err,
    };
    let workspace = match resolve_workspace(&conn, Some(workspace_id)) {
        Ok(workspace) => workspace,
        Err(err) => return err,
    };
    let project = match resolve_project(&conn, &workspace, Some(project_id)) {
        Ok(project) => project,
        Err(err) => return err,
    };
    if let Err(err) = ensure_workspace_enabled(&workspace) {
        return err;
    }
    if !parent.is_empty() {
        match folder_known(&conn, &project.id, &parent) {
            Ok(true) => {}
            Ok(false) => return bad_request("Unknown folder"),
            Err(err) => return internal(err),
        }
    }
    match folder_known(&conn, &project.id, &path) {
        Ok(true) => return conflict("Folder already exists"),
        Ok(false) => {}
        Err(err) => return internal(err),
    }

    let root = match workspace_root(&workspace) {
        Ok(root) => root,
        Err(err) => return err,
    };
    let raw = match project_raw(&root, &project.id) {
        Ok(raw) => raw,
        Err(err) => return err,
    };
    let dir = raw.join(&path);
    if dir.exists() {
        return conflict("Folder already exists");
    }
    if let Some(parent_dir) = dir.parent() {
        if parent_dir != raw && !parent_dir.is_dir() {
            return bad_request("Unknown folder");
        }
    }
    if let Err(err) = fs::create_dir_all(&raw) {
        return internal(err.to_string());
    }
    if let Err(err) = fs::create_dir(&dir) {
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            return conflict("Folder already exists");
        }
        return internal(err.to_string());
    }
    if let Err(err) = insert_folder(&conn, &project.id, &path) {
        let _ = fs::remove_dir(&dir);
        return internal(err);
    }
    json!({ "ok": true, "workspace_id": workspace.id, "project_id": project.id, "path": path })
}

/// `{ workspace_id, project_id, path }` → `{ ok, project_id, path }`.
/// Removes one empty folder. A document, a child folder, or any other file keeps it.
pub fn delete_folder(payload: &Value) -> Value {
    let Some(workspace_id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let Some(path) = text_field(payload, "path") else {
        return bad_request("Missing folder");
    };
    if !folder_path_ok(path) {
        return bad_request("Invalid folder");
    }
    let conn = match open_data() {
        Ok(conn) => conn,
        Err(err) => return err,
    };
    let workspace = match resolve_workspace(&conn, Some(workspace_id)) {
        Ok(workspace) => workspace,
        Err(err) => return err,
    };
    let project = match resolve_project(&conn, &workspace, Some(project_id)) {
        Ok(project) => project,
        Err(err) => return err,
    };
    if let Err(err) = ensure_workspace_enabled(&workspace) {
        return err;
    }
    match folder_busy(&conn, &project.id, path) {
        Ok(true) => return conflict("Folder is not empty"),
        Ok(false) => {}
        Err(err) => return internal(err),
    }
    let root = match workspace_root(&workspace) {
        Ok(root) => root,
        Err(err) => return err,
    };
    let raw = match project_raw(&root, &project.id) {
        Ok(raw) => raw,
        Err(err) => return err,
    };
    let dir = raw.join(path);
    if dir_occupied(&dir) {
        return conflict("Folder is not empty");
    }
    let had_row = match delete_folder_row(&conn, &project.id, path) {
        Ok(had) => had,
        Err(err) => return internal(err),
    };
    if !dir.exists() {
        return if had_row { ok_folder(&workspace.id, &project.id, path) } else { not_found("Unknown folder") };
    }
    if let Err(err) = fs::remove_dir(&dir) {
        if had_row {
            let _ = insert_folder(&conn, &project.id, path);
        }
        return internal(err.to_string());
    }
    ok_folder(&workspace.id, &project.id, path)
}

fn ok_folder(workspace_id: &str, project_id: &str, path: &str) -> Value {
    json!({ "ok": true, "workspace_id": workspace_id, "project_id": project_id, "path": path })
}

/// After a document is deleted, remember its parent when no document remains inside it.
/// The directory itself is left on disk.
pub(crate) fn keep_parent_folder(conn: &Connection, project_id: &str, raw_path: &str) -> Result<(), String> {
    let Some((parent, _)) = raw_path.rsplit_once('/') else {
        return Ok(());
    };
    if !folder_path_ok(parent) || docs_inside(conn, project_id, parent)? {
        return Ok(());
    }
    insert_folder(conn, project_id, parent)
}

pub(crate) fn list_folders(conn: &Connection, workspace: &WorkspaceRow) -> Result<Vec<(String, String)>, String> {
    if workspace.disabled_at.is_some() {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT folder.project_id, folder.path
             FROM folder
             JOIN project ON project.id = folder.project_id
             WHERE project.workspace_id = ?1
             ORDER BY folder.project_id, folder.path",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![workspace.id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn parent_path(payload: &Value) -> Result<String, Value> {
    match text_field(payload, "parent") {
        None => Ok(String::new()),
        Some("/") => Ok(String::new()),
        Some(path) if folder_path_ok(path) => Ok(path.to_string()),
        Some(_) => Err(bad_request("Invalid folder")),
    }
}

pub(super) fn one_segment(name: &str) -> bool {
    !name.contains('/') && !name.contains('\\') && folder_path_ok(name)
}

pub(super) fn folder_path_ok(path: &str) -> bool {
    !path.is_empty() && !path.starts_with('/') && !path.ends_with('/') && is_valid_common_path(&format!("{path}/kept.md"))
}

pub(super) fn folder_known(conn: &Connection, project_id: &str, path: &str) -> Result<bool, String> {
    Ok(folder_row(conn, project_id, path)? || docs_inside(conn, project_id, path)?)
}

fn folder_row(conn: &Connection, project_id: &str, path: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM folder WHERE project_id = ?1 AND path = ?2)",
        params![project_id, path],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn folder_busy(conn: &Connection, project_id: &str, path: &str) -> Result<bool, String> {
    Ok(docs_inside(conn, project_id, path)? || child_folder(conn, project_id, path)?)
}

fn child_folder(conn: &Connection, project_id: &str, path: &str) -> Result<bool, String> {
    let like = format!("{}/%", escape_like(path));
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM folder WHERE project_id = ?1 AND path LIKE ?2 ESCAPE '\\')",
        params![project_id, like],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn delete_folder_row(conn: &Connection, project_id: &str, path: &str) -> Result<bool, String> {
    let removed = conn
        .execute("DELETE FROM folder WHERE project_id = ?1 AND path = ?2", params![project_id, path])
        .map_err(|e| e.to_string())?;
    Ok(removed > 0)
}

/// A missing directory is free. A file, a child entry, or an unreadable directory is occupied.
fn dir_occupied(dir: &Path) -> bool {
    let Ok(mut entries) = fs::read_dir(dir) else {
        return dir.exists();
    };
    entries.next().is_some()
}

fn docs_inside(conn: &Connection, project_id: &str, path: &str) -> Result<bool, String> {
    let like = format!("{}/%", escape_like(path));
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM document
            WHERE project_id = ?1 AND raw_path LIKE ?2 ESCAPE '\\'
         )",
        params![project_id, like],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub(super) fn escape_like(path: &str) -> String {
    path.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn insert_folder(conn: &Connection, project_id: &str, path: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO folder (project_id, path) VALUES (?1, ?2)",
        params![project_id, path],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub(super) fn project_raw(root: &Path, project_id: &str) -> Result<PathBuf, Value> {
    if project_id.is_empty() || project_id.contains("..") || project_id.contains('/') || project_id.contains('\\') {
        return Err(bad_request("Invalid project id"));
    }
    let root = root.canonicalize().map_err(|e| internal(e.to_string()))?;
    Ok(root.join("raw").join(project_id))
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_folder.rs"]
mod tests;
