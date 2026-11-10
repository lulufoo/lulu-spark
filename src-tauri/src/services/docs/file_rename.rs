//! Rename one document's markdown file inside its folder.
//!
//! The raw file, translation files, and a digest that still uses that filename move
//! together. The title does not follow. A later failure puts the files back.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::archive_parse::is_valid_common_path;
use crate::services::search_index;
use crate::services::spark_data::{
    get_doc, is_valid_doc_id, list_docs, translation_common_paths, update_doc, DocRow,
};

use super::create_meta::normalize_markdown_filename;
use super::folder_rename_disk::{move_rel, same_dir, undo_files};
use super::resolve::{
    bad_request, conflict, ensure_workspace_enabled, internal, not_found, open_data, resolve_project,
    resolve_workspace, text_field, workspace_root,
};
use super::store::doc_file_path;

/// `{ workspace_id, project_id, id, name }` → `{ ok, id, raw_path }`.
/// `name` is the new markdown basename in the same folder.
pub fn rename_doc(payload: &Value) -> Value {
    let Some(workspace_id) = text_field(payload, "workspace_id") else {
        return bad_request("Missing workspace_id");
    };
    let Some(project_id) = text_field(payload, "project_id") else {
        return bad_request("Missing project_id");
    };
    let Some(id) = text_field(payload, "id") else {
        return bad_request("Missing id");
    };
    let Some(name) = text_field(payload, "name") else {
        return bad_request("Missing filename");
    };
    if !is_valid_doc_id(id) {
        return bad_request("Invalid id");
    }
    let filename = match normalize_markdown_filename(name) {
        Ok(name) => name,
        Err(err) => return err,
    };

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
    let new_raw = replace_name(&doc.raw_path, &filename);
    if !is_valid_common_path(&new_raw) {
        return bad_request("Invalid filename");
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
        let old_trans = translation_common_paths(doc).map_err(internal)?;
        let new_trans = translation_common_paths(&next).map_err(internal)?;
        for ((_, from), (_, to)) in old_trans.iter().zip(new_trans.iter()) {
            move_rel(root, "raw", project, from, to, files)?;
        }
    }
    if let Some(digest) = &doc.digest_path {
        if basename(digest) == basename(&doc.raw_path) {
            let dest = replace_name(digest, basename(new_raw));
            if move_files {
                move_rel(root, "digest", project, digest, &dest, files)?;
            }
            next.digest_path = Some(dest);
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

fn replace_name(rel: &str, name: &str) -> String {
    match rel.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{name}"),
        None => name.to_string(),
    }
}

fn basename(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_file_rename.rs"]
mod tests;
