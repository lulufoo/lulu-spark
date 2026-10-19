//! Migration step: every `doc_id` becomes `{workspace.id}-{32 hex}`.
//!
//! Plain 32 hex rows and rows whose prefix is not this workspace's current id
//! are rewritten. Already-canonical ids are skipped. `find_doc` still accepts
//! the old hex so existing links keep working.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde_json::Value;

use super::doc_id::prefixed_doc_id;
use super::doc_row::{get_doc, list_docs};
use super::doc_table::validate_doc_table_name;
use super::workspace_row::list_all_workspaces;
use crate::config::paths;

/// Sentinel `MIGRATIONS` entry. Not executable SQL; `migrate` intercepts it.
pub(super) const PREFIX_LEGACY_DOC_IDS_STEP: &str = "-- prefix legacy doc ids with workspace id";

/// `(old id, new id)` rewritten in this step.
pub(super) fn prefix_legacy_doc_ids(conn: &Connection, db_path: &Path) -> Result<(), String> {
    let mut rewritten = Vec::new();
    for workspace in list_all_workspaces(conn)? {
        if !table_exists(conn, &workspace.doc_table)? {
            continue;
        }
        rewritten.extend(rewrite_table(conn, &workspace.id, &workspace.doc_table)?);
    }
    remap_cache_doc_ids(db_path, &rewritten)?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    conn.prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")
        .and_then(|mut stmt| stmt.exists([name]))
        .map_err(|e| e.to_string())
}

fn rewrite_table(
    conn: &Connection,
    workspace_id: &str,
    table: &str,
) -> Result<Vec<(String, String)>, String> {
    validate_doc_table_name(table)?;
    let mut out = Vec::new();
    for doc in list_docs(conn, table)? {
        let Some(new_id) = prefixed_doc_id(workspace_id, &doc.doc_id) else {
            continue;
        };
        if new_id == doc.doc_id {
            continue;
        }
        if get_doc(conn, table, &new_id)?.is_some() {
            return Err(format!("cannot prefix {}: {new_id} already exists", doc.doc_id));
        }
        conn.execute(
            &format!("UPDATE {table} SET doc_id = ?1 WHERE doc_id = ?2"),
            params![new_id, doc.doc_id],
        )
        .map_err(|e| e.to_string())?;
        out.push((doc.doc_id, new_id));
    }
    Ok(out)
}

fn remap_cache_doc_ids(db_path: &Path, rewritten: &[(String, String)]) -> Result<(), String> {
    if rewritten.is_empty() {
        return Ok(());
    }
    let Some(cache) = cache_dir_for_opened_db(db_path) else {
        return Ok(());
    };
    remap_search_index_doc_ids(&cache.join("search-index.sqlite"), rewritten)?;
    remap_viewer_state_docs(&cache.join("workspace-viewer-state"), rewritten)?;
    Ok(())
}

fn cache_dir_for_opened_db(opened_db: &Path) -> Option<PathBuf> {
    let prod = paths::spark_data_db_path().ok()?;
    let opened = opened_db.canonicalize().unwrap_or_else(|_| opened_db.to_path_buf());
    let prod = prod.canonicalize().unwrap_or(prod);
    (opened == prod).then(|| paths::cache_dir().ok())?
}

fn remap_search_index_doc_ids(index: &Path, rewritten: &[(String, String)]) -> Result<(), String> {
    if !index.is_file() {
        return Ok(());
    }
    let conn = Connection::open(index).map_err(|e| e.to_string())?;
    let has_chunk: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'chunk'")
        .and_then(|mut stmt| stmt.exists([]))
        .unwrap_or(false);
    if !has_chunk {
        return Ok(());
    }
    for (old, new) in rewritten {
        conn.execute("UPDATE chunk SET doc_id = ?1 WHERE doc_id = ?2", params![new, old])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn remap_viewer_state_docs(dir: &Path, rewritten: &[(String, String)]) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        rewrite_viewer_file(&path, rewritten)?;
    }
    Ok(())
}

fn rewrite_viewer_file(path: &Path, rewritten: &[(String, String)]) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let Some(projects) = value.get_mut("projects").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    let mut changed = false;
    for project in projects.values_mut() {
        let Some(doc) = project.get("doc").and_then(Value::as_str) else {
            continue;
        };
        if let Some((_, new)) = rewritten.iter().find(|(old, _)| old == doc) {
            project["doc"] = Value::String(new.clone());
            changed = true;
        }
    }
    if changed {
        fs::write(path, serde_json::to_string(&value).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
