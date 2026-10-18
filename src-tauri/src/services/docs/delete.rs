//! `delete_doc`: remove a document's files, then its row.

use std::fs;

use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::docs::store::doc_file_path;
use crate::services::search_index;
use crate::services::spark_data::{delete_doc as delete_doc_row, delete_unused_tags};

use super::resolve::{
    bad_request, doc_id_field, internal, open_data, resolve_doc, resolve_doc_workspace,
    workspace_root,
};

/// `{ workspace_id?, id }` → `{ ok, id, removed }`; `removed` counts the files deleted.
pub fn delete_doc(payload: &Value) -> Value {
    let id = match doc_id_field(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let conn = match open_data() {
        Ok(c) => c,
        Err(v) => return v,
    };
    let workspace = match resolve_doc_workspace(&conn, payload, &id) {
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
    if let Err(e) = delete_doc_row(&conn, &workspace.doc_table, &id)
        .and_then(|_| delete_unused_tags(&conn).map(|_| ()))
    {
        return internal(e);
    }
    search_index::sync_workspace_docs_best_effort(&workspace.id, &[id.as_str()]);
    json!({ "ok": true, "id": id, "removed": removed })
}
