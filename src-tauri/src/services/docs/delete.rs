//! `delete_doc`: remove a document's cloud copy, then its files, then its row.

use std::fs;

use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::cloud_sync::{self, delete_cloud_copy, CloudDocs, CloudSyncError};
use crate::services::docs::attached::remove_attached_dir;
use crate::services::docs::store::doc_file_path;
use crate::services::search_index;
use crate::services::spark_data::{delete_doc as delete_doc_row, delete_unused_tags};

use super::resolve::{
    bad_request, doc_id_field, internal, open_data, resolve_doc, resolve_workspace_from_doc_id,
    workspace_root,
};

/// The error when the cloud copy could not be removed. The document is still there, on this
/// machine and in the cloud, so the user can try again.
fn cloud_failed(error: &CloudSyncError) -> Value {
    json!({
        "error": format!("Could not remove the document from the cloud, so it was not deleted: {error}"),
        "_status": 502,
    })
}

/// `{ id }` → `{ ok, id, removed }`; `removed` counts the files deleted. `workspace_id` is refused.
///
/// A signed-in user's document is removed from the cloud first, and only when that succeeds are
/// the files and the row deleted: a failed cloud call (offline, session expired, ...) fails the
/// whole delete and keeps the document. A user who is not signed in deletes on this machine only.
pub fn delete_doc(payload: &Value) -> Value {
    delete_doc_with(payload, cloud_sync::shared())
}

fn delete_doc_with(payload: &Value, cloud: &dyn CloudDocs) -> Value {
    if payload.get("workspace_id").is_some() {
        return bad_request("workspace_id is not supported on delete_doc");
    }
    let id = match doc_id_field(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let workspace = match resolve_workspace_from_doc_id(&conn, &id) {
        Ok(w) => w,
        Err(v) => return v,
    };
    let (doc, project) = match resolve_doc(&conn, &workspace, &id) {
        Ok(found) => found,
        Err(v) => return v,
    };
    let rel = doc.raw_path.trim();
    if rel.is_empty() || rel.contains("..") {
        return bad_request("Invalid path");
    }
    let root = match workspace_root(&workspace) {
        Ok(r) => r,
        Err(v) => return v,
    };
    if let Err(e) = delete_cloud_copy(cloud, &id) {
        return cloud_failed(&e);
    }

    let mut files = vec![("raw", rel.to_string()), ("digest", rel.to_string())];
    for lang in doc.translations.as_array().into_iter().flatten().filter_map(Value::as_str) {
        if let Some(path) = expected_lang_common_path(rel, lang) {
            files.push(("raw", path));
        }
    }
    let mut removed = 0;
    for (layer, rel) in &files {
        let Ok(target) = doc_file_path(&root, layer, &project.id, rel) else {
            continue;
        };
        if target.is_file() && fs::remove_file(&target).is_ok() {
            removed += 1;
        }
    }
    if let Ok(raw_file) = doc_file_path(&root, "raw", &project.id, rel) {
        if let Some(md_dir) = raw_file.parent() {
            let _ = remove_attached_dir(md_dir, &id);
        }
    }
    if let Err(e) = delete_doc_row(&conn, &id)
        .and_then(|_| delete_unused_tags(&conn).map(|_| ()))
    {
        return internal(e);
    }
    search_index::sync_workspace_docs_best_effort(&workspace.id, &[id.as_str()]);
    json!({ "ok": true, "id": id, "removed": removed })
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_delete.rs"]
mod tests;
