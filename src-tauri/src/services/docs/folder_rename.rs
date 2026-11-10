//! Rename one folder by moving each document, then the folder rows.
//!
//! A document's files and its row move together. A later failure puts every document
//! that already moved back where it was. Child folders follow the new name.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::services::search_index;
use crate::services::spark_data::{list_docs, translation_common_paths, update_doc, DocRow};

use super::folder::{escape_like, folder_known, folder_path_ok, one_segment, project_raw};
use super::folder_rename_disk::{move_attached, move_rel, remove_empty, same_dir, shift_tree, undo_files};
use super::resolve::{
    bad_request, conflict, ensure_workspace_enabled, internal, not_found, open_data, resolve_project,
    resolve_workspace, text_field, workspace_root,
};

struct MovedDoc {
    original: DocRow,
    files: Vec<(PathBuf, PathBuf)>,
}

/// `{ workspace_id, project_id, path, name }` → `{ ok, project_id, path }`.
/// `path` is the folder to rename. `name` is its new single segment, under the same parent.
pub fn rename_folder(payload: &Value) -> Value {
    let Some(workspace_id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let Some(path) = text_field(payload, "path") else {
        return bad_request("Missing folder");
    };
    let Some(name) = text_field(payload, "name") else {
        return bad_request("Missing folder name");
    };
    if !folder_path_ok(path) || !one_segment(name) {
        return bad_request("Invalid folder");
    }
    let new_path = match path.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{name}"),
        None => name.to_string(),
    };
    if !folder_path_ok(&new_path) {
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
    let root = match workspace_root(&workspace) {
        Ok(root) => root,
        Err(err) => return err,
    };
    let raw = match project_raw(&root, &project.id) {
        Ok(raw) => raw,
        Err(err) => return err,
    };
    let docs = match docs_under(&conn, &workspace.id, &project.id, path) {
        Ok(docs) => docs,
        Err(err) => return internal(err),
    };
    let listed = match folder_paths(&conn, &project.id, path) {
        Ok(paths) => paths,
        Err(err) => return internal(err),
    };
    let old_dir = raw.join(path);
    if docs.is_empty() && listed.is_empty() && !old_dir.exists() {
        return not_found("Unknown folder");
    }
    if new_path == path {
        return json!({ "ok": true, "workspace_id": workspace.id, "project_id": project.id, "path": path });
    }
    let new_dir = raw.join(&new_path);
    let same = same_dir(&old_dir, &new_dir);
    match dest_taken(&conn, &project.id, &new_path, &new_dir, same) {
        Ok(true) => return conflict("Folder already exists"),
        Ok(false) => {}
        Err(err) => return internal(err),
    }

    let mut moved = Vec::new();
    for doc in docs.iter().cloned() {
        let mut files = Vec::new();
        let next = match retarget_doc(&root, &project.id, &doc, path, &new_path, !same, &mut files) {
            Ok(next) => next,
            Err(err) => {
                undo_files(&files);
                undo_docs(&conn, &moved);
                return err;
            }
        };
        match update_doc(&conn, &next) {
            Ok(true) => {}
            Ok(false) => {
                undo_files(&files);
                undo_docs(&conn, &moved);
                return internal("document missing");
            }
            Err(err) => {
                undo_files(&files);
                undo_docs(&conn, &moved);
                return internal(err);
            }
        }
        moved.push(MovedDoc { original: doc, files });
    }
    let folders = match retarget_folders(&conn, &project.id, path, &new_path) {
        Ok(rows) => rows,
        Err(err) => {
            undo_docs(&conn, &moved);
            return internal(err);
        }
    };
    if !same {
        let mut leftover = Vec::new();
        if let Err(err) = shift_tree(&old_dir, &new_dir, &mut leftover) {
            undo_files(&leftover);
            restore_folders(&conn, &project.id, &folders);
            undo_docs(&conn, &moved);
            let _ = remove_empty(&new_dir);
            return internal(err);
        }
        remove_empty(&old_dir);
        if old_dir.exists() {
            undo_files(&leftover);
            restore_folders(&conn, &project.id, &folders);
            undo_docs(&conn, &moved);
            let _ = remove_empty(&new_dir);
            return conflict("Folder is not empty");
        }
    }
    let ids: Vec<&str> = moved.iter().map(|item| item.original.doc_id.as_str()).collect();
    if !ids.is_empty() {
        search_index::sync_workspace_docs_best_effort(&workspace.id, &ids);
    }
    json!({ "ok": true, "workspace_id": workspace.id, "project_id": project.id, "path": new_path })
}

fn retarget_doc(
    root: &Path,
    project: &str,
    doc: &DocRow,
    from: &str,
    to: &str,
    move_files: bool,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<DocRow, Value> {
    let new_raw = shift_prefix(&doc.raw_path, from, to).ok_or_else(|| bad_request("Invalid folder"))?;
    if move_files {
        move_rel(root, "raw", project, &doc.raw_path, &new_raw, files)?;
        for (_, rel) in translation_common_paths(doc).map_err(internal)? {
            let dest = shift_prefix(&rel, from, to).ok_or_else(|| bad_request("Invalid folder"))?;
            move_rel(root, "raw", project, &rel, &dest, files)?;
        }
        move_attached(root, project, &doc.doc_id, &doc.raw_path, &new_raw, files)?;
    }
    let mut next = doc.clone();
    next.raw_path = new_raw;
    if let Some(digest) = &doc.digest_path {
        if let Some(dest) = shift_prefix(digest, from, to) {
            if move_files {
                move_rel(root, "digest", project, digest, &dest, files)?;
            }
            next.digest_path = Some(dest);
        }
    }
    Ok(next)
}

fn docs_under(conn: &Connection, workspace: &str, project: &str, folder: &str) -> Result<Vec<DocRow>, String> {
    let mut docs = list_docs(conn, workspace)?;
    docs.retain(|doc| doc.project_id == project && shift_prefix(&doc.raw_path, folder, folder).is_some());
    docs.sort_by(|a, b| a.raw_path.cmp(&b.raw_path));
    Ok(docs)
}

fn folder_paths(conn: &Connection, project: &str, folder: &str) -> Result<Vec<String>, String> {
    let like = format!("{}/%", escape_like(folder));
    let mut stmt = conn
        .prepare("SELECT path FROM folder WHERE project_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')")
        .map_err(|err| err.to_string())?;
    let rows = stmt.query_map(params![project, folder, like], |row| row.get(0)).map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|err| err.to_string())
}

fn retarget_folders(conn: &Connection, project: &str, from: &str, to: &str) -> Result<Vec<(String, String)>, String> {
    let mut paths = folder_paths(conn, project, from)?;
    paths.sort_by(|a, b| b.len().cmp(&a.len()));
    let mut changed = Vec::new();
    for old in paths {
        let Some(new) = shift_prefix(&old, from, to) else { continue };
        let n = conn
            .execute("UPDATE folder SET path = ?3 WHERE project_id = ?1 AND path = ?2", params![project, old, new])
            .map_err(|err| err.to_string())?;
        if n > 0 {
            changed.push((old, new));
        }
    }
    Ok(changed)
}

fn dest_taken(conn: &Connection, project: &str, path: &str, dir: &Path, same: bool) -> Result<bool, String> {
    if same {
        return Ok(false);
    }
    Ok(dir.exists() || folder_known(conn, project, path)? || !folder_paths(conn, project, path)?.is_empty())
}

fn shift_prefix(path: &str, from: &str, to: &str) -> Option<String> {
    let rest = path.strip_prefix(from)?;
    if rest.is_empty() {
        return Some(to.to_string());
    }
    rest.strip_prefix('/').map(|tail| format!("{to}/{tail}"))
}

fn undo_docs(conn: &Connection, moved: &[MovedDoc]) {
    for item in moved.iter().rev() {
        undo_files(&item.files);
        let _ = update_doc(conn, &item.original);
    }
}

fn restore_folders(conn: &Connection, project: &str, changed: &[(String, String)]) {
    for (old, new) in changed.iter().rev() {
        let _ = conn.execute(
            "UPDATE folder SET path = ?3 WHERE project_id = ?1 AND path = ?2",
            params![project, new, old],
        );
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_folder_rename.rs"]
mod tests;
