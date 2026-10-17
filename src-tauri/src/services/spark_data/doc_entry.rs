//! A document row in the shape of the docs-index entry, and lookup of a document by id.

use rusqlite::Connection;
use serde_json::{json, Map, Value};

use crate::services::archive_parse::expected_lang_common_path;

use super::doc_id::split_doc_id;
use super::doc_row::{get_doc, list_docs, DocRow};
use super::project_row::get_project;
use super::workspace_row::{get_workspace, list_workspaces, WorkspaceRow};

/// A document together with the workspace whose table holds it.
#[derive(Debug, Clone)]
pub struct FoundDoc {
    pub workspace: WorkspaceRow,
    pub doc: DocRow,
}

/// The document with this id. A prefixed id (`<workspace id>-<hex>`) goes straight to that
/// workspace's table; a plain 32 hex id from before the prefix scans the tables in order and
/// returns the first match (nothing enforces uniqueness across tables).
pub fn find_doc(conn: &Connection, id: &str) -> Result<Option<FoundDoc>, String> {
    if let Some((Some(workspace_id), _)) = split_doc_id(id) {
        let Some(workspace) = get_workspace(conn, workspace_id)? else {
            return Ok(None);
        };
        return Ok(get_doc(conn, &workspace.doc_table, id)?.map(|doc| FoundDoc { workspace, doc }));
    }
    for workspace in list_workspaces(conn)? {
        if let Some(doc) = get_doc(conn, &workspace.doc_table, id)? {
            return Ok(Some(FoundDoc { workspace, doc }));
        }
    }
    Ok(None)
}

/// Folder of the project `project_id`; the folder is the middle path segment on disk.
pub fn project_folder(conn: &Connection, project_id: &str) -> Result<String, String> {
    get_project(conn, project_id)?
        .map(|project| project.folder)
        .ok_or_else(|| format!("project {project_id} missing"))
}

/// `translations` is stored as a language list; each path is derived from `raw_path`.
pub fn translation_common_paths(doc: &DocRow) -> Result<Vec<(String, String)>, String> {
    let langs = doc.translations.as_array().cloned().unwrap_or_default();
    langs
        .iter()
        .filter_map(Value::as_str)
        .map(|lang| {
            expected_lang_common_path(&doc.raw_path, lang)
                .map(|path| (lang.to_string(), path))
                .ok_or_else(|| format!("doc {}: no translation path for `{}`", doc.doc_id, doc.raw_path))
        })
        .collect()
}

/// `{id, workspace, project_id, folder, created_at, layers, source_type, translations?, title?}`.
/// `folder` is the project's folder.
pub fn entry_from_doc(doc: &DocRow, workspace_id: &str, folder: &str) -> Result<Value, String> {
    let mut entry = Map::new();
    entry.insert("id".into(), json!(doc.doc_id));
    entry.insert("workspace".into(), json!(workspace_id));
    entry.insert("project_id".into(), json!(doc.project_id));
    entry.insert("folder".into(), json!(folder));
    entry.insert(
        "layers".into(),
        if doc.digest_path.is_some() { json!(["raw", "digest"]) } else { json!(["raw"]) },
    );
    for (key, value) in [
        ("created_at", &doc.created_at),
        ("source_type", &doc.source_type),
        ("title", &doc.title),
    ] {
        if let Some(v) = value {
            entry.insert(key.into(), json!(v));
        }
    }
    let translations = translation_common_paths(doc)?;
    if !translations.is_empty() {
        let map: Map<String, Value> = translations.into_iter().map(|(lang, path)| (lang, json!(path))).collect();
        entry.insert("translations".into(), Value::Object(map));
    }
    Ok(Value::Object(entry))
}

/// `doc_id → entry` for every document of the workspace.
pub fn list_doc_entries(conn: &Connection, workspace: &WorkspaceRow) -> Result<Map<String, Value>, String> {
    let mut entries = Map::new();
    for doc in list_docs(conn, &workspace.doc_table)? {
        let folder = project_folder(conn, &doc.project_id)?;
        entries.insert(doc.doc_id.clone(), entry_from_doc(&doc, &workspace.id, &folder)?);
    }
    Ok(entries)
}
