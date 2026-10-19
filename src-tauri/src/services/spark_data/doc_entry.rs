//! A document row in the shape of the docs-index entry, and lookup of a document by id.

use std::collections::HashSet;

use rusqlite::Connection;
use serde_json::{json, Map, Value};

use crate::services::archive_parse::expected_lang_common_path;

use super::doc_row::{get_doc, list_docs, DocRow};
use super::project_row::{get_project, list_projects};
use super::workspace_row::{get_workspace, WorkspaceRow};

/// A document together with the workspace its project belongs to.
#[derive(Debug, Clone)]
pub struct FoundDoc {
    pub workspace: WorkspaceRow,
    pub doc: DocRow,
}

/// The document with this id, with the workspace of its project. Disabled workspaces stay
/// searchable by id: a known id keeps opening.
pub fn find_doc(conn: &Connection, id: &str) -> Result<Option<FoundDoc>, String> {
    let Some(doc) = get_doc(conn, id)? else {
        return Ok(None);
    };
    let project = get_project(conn, &doc.project_id)?
        .ok_or_else(|| format!("project {} missing", doc.project_id))?;
    let workspace = get_workspace(conn, &project.workspace_id)?
        .ok_or_else(|| format!("workspace {} missing", project.workspace_id))?;
    Ok(Some(FoundDoc { workspace, doc }))
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

/// `{id, workspace_id, project_id, raw_path, created_at, layers, source_type, translations?, title?}`.
/// `raw_path` is the document-internal path and keeps the original directory structure.
pub fn entry_from_doc(doc: &DocRow, workspace_id: &str) -> Result<Value, String> {
    let mut entry = Map::new();
    entry.insert("id".into(), json!(doc.doc_id));
    entry.insert("workspace_id".into(), json!(workspace_id));
    entry.insert("project_id".into(), json!(doc.project_id));
    entry.insert("raw_path".into(), json!(doc.raw_path));
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

/// `doc_id → entry` for every document of the workspace that is still shown: none for a
/// disabled workspace. Those stay readable by id.
pub fn list_doc_entries(conn: &Connection, workspace: &WorkspaceRow) -> Result<Map<String, Value>, String> {
    let mut entries = Map::new();
    if workspace.disabled_at.is_some() {
        return Ok(entries);
    }
    let projects: HashSet<String> = list_projects(conn, &workspace.id)?
        .into_iter()
        .map(|p| p.id)
        .collect();
    for doc in list_docs(conn, &workspace.id)? {
        if !projects.contains(&doc.project_id) {
            return Err(format!("project {} missing", doc.project_id));
        }
        entries.insert(doc.doc_id.clone(), entry_from_doc(&doc, &workspace.id)?);
    }
    Ok(entries)
}
