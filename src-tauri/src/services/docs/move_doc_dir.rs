//! Move one document into another folder of the same project.
//!
//! The raw file, translation files, attached directory, and a digest living beside the
//! body move together. A later failure puts the files back. The folder the document
//! leaves is kept in the tree when nothing is left inside it.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::archive_parse::{expected_lang_common_path, is_valid_common_path};
use crate::services::search_index;
use crate::services::spark_data::{
    get_doc, is_valid_doc_id, list_docs, translation_common_paths, update_doc, DocRow,
};

use super::folder::{folder_known, folder_path_ok, keep_parent_folder};
use super::folder_rename_disk::{move_attached, move_rel, same_dir, undo_files};
use super::resolve::{
    bad_request, conflict, ensure_workspace_enabled, internal, not_found, open_data, resolve_project,
    resolve_workspace, text_field, workspace_root,
};
use super::store::doc_file_path;

/// `{ workspace_id, project_id, id, target }` → `{ ok, id, raw_path }`.
/// `target` is the destination folder relative to the project, such as `plans/q1`;
/// omitted or empty moves the document to the project root.
pub fn move_doc_dir(payload: &Value) -> Value {
    let Some(workspace_id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    if !is_valid_doc_id(id) {
        return bad_request("Invalid id");
    }
    let target = text_field(payload, "target").unwrap_or_default();
    if !target.is_empty() && !folder_path_ok(&target) {
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
    let doc = match get_doc(&conn, id) {
        Ok(Some(doc)) if doc.project_id == project.id => doc,
        Ok(_) => return not_found("Unknown document"),
        Err(err) => return internal(err),
    };
    if !target.is_empty() {
        match folder_known(&conn, &project.id, &target) {
            Ok(true) => {}
            Ok(false) => return bad_request("Unknown folder"),
            Err(err) => return internal(err),
        }
    }
    let new_raw = join_dir(&target, basename(&doc.raw_path));
    if !is_valid_common_path(&new_raw) {
        return bad_request("Invalid folder");
    }
    if new_raw == doc.raw_path {
        return json!({ "ok": true, "id": doc.doc_id, "raw_path": doc.raw_path });
    }

    let root = match workspace_root(&workspace) {
        Ok(root) => root,
        Err(err) => return err,
    };
    let src = match doc_file_path(&root, "raw", &project.id, &doc.raw_path) {
        Ok(path) => path,
        Err(err) => return err,
    };
    let dest = match doc_file_path(&root, "raw", &project.id, &new_raw) {
        Ok(path) => path,
        Err(err) => return err,
    };
    let same = same_dir(&src, &dest);
    match dest_taken(&conn, &workspace.id, &project.id, &doc.doc_id, &new_raw, &dest, same) {
        Ok(true) => return conflict("File already exists"),
        Ok(false) => {}
        Err(err) => return internal(err),
    }

    let mut files = Vec::new();
    let next = match retarget(&root, &project.id, &doc, &new_raw, !same, &mut files) {
        Ok(next) => next,
        Err(err) => {
            undo_files(&files);
            return err;
        }
    };
    match update_doc(&conn, &next) {
        Ok(true) => {}
        Ok(false) => {
            undo_files(&files);
            return internal("document missing");
        }
        Err(err) => {
            undo_files(&files);
            return internal(err);
        }
    }
    if let Err(err) = keep_parent_folder(&conn, &project.id, &doc.raw_path) {
        return internal(err);
    }
    search_index::sync_workspace_docs_best_effort(&workspace.id, &[doc.doc_id.as_str()]);
    json!({ "ok": true, "id": doc.doc_id, "raw_path": next.raw_path })
}

fn retarget(
    root: &Path,
    project: &str,
    doc: &DocRow,
    new_raw: &str,
    move_files: bool,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<DocRow, Value> {
    let mut next = doc.clone();
    next.raw_path = new_raw.to_string();
    if move_files {
        move_rel(root, "raw", project, &doc.raw_path, new_raw, files)?;
        for (lang, rel) in translation_common_paths(doc).map_err(internal)? {
            let dest =
                expected_lang_common_path(new_raw, &lang).ok_or_else(|| bad_request("Invalid folder"))?;
            move_rel(root, "raw", project, &rel, &dest, files)?;
        }
        move_attached(root, project, &doc.doc_id, &doc.raw_path, new_raw, files)?;
        if let Some(digest) = &doc.digest_path {
            if dir_of(digest) == dir_of(&doc.raw_path) {
                let dest = join_dir(dir_of(new_raw).unwrap_or_default(), basename(digest));
                move_rel(root, "digest", project, digest, &dest, files)?;
                next.digest_path = Some(dest);
            }
        }
    }
    Ok(next)
}

fn dest_taken(
    conn: &Connection,
    workspace: &str,
    project: &str,
    doc_id: &str,
    new_raw: &str,
    dest: &Path,
    same: bool,
) -> Result<bool, String> {
    let taken = list_docs(conn, workspace)?
        .iter()
        .any(|doc| doc.project_id == project && doc.doc_id != doc_id && doc.raw_path == new_raw);
    if taken {
        return Ok(true);
    }
    if same {
        return Ok(false);
    }
    Ok(dest.exists())
}

fn join_dir(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn dir_of(rel: &str) -> Option<&str> {
    rel.rsplit_once('/').map(|(parent, _)| parent)
}

fn basename(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_move_dir.rs"]
mod tests;
