//! Document ids.
//!
//! A new document id is `<workspace id>-<32 lowercase hex>`. The workspace id never changes and
//! has no `-` (it is `[a-z0-9_]`), so the first `-` splits the id and the prefix names the
//! document table. Ids created before this format are plain 32 hex and are found by scanning
//! every table, so both shapes are valid.

use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::services::id::random_entry_id;

use super::doc_table::{validate_doc_table_name, validate_workspace_id};
use super::workspace_row::list_all_workspaces;

fn is_hex32(s: &str) -> bool {
    s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// A fresh id for a document of `workspace_id`.
pub fn new_doc_id(workspace_id: &str) -> String {
    format!("{workspace_id}-{}", random_entry_id())
}

/// `(workspace id, hex)` of a prefixed id, `(None, hex)` of a plain 32 hex id, `None` otherwise.
pub fn split_doc_id(id: &str) -> Option<(Option<&str>, &str)> {
    if is_hex32(id) {
        return Some((None, id));
    }
    let (workspace, hex) = id.split_once('-')?;
    (validate_workspace_id(workspace).is_ok() && is_hex32(hex)).then_some((Some(workspace), hex))
}

pub fn is_valid_doc_id(id: &str) -> bool {
    split_doc_id(id).is_some()
}

/// Document ids present in more than one workspace table, with the workspaces that hold them.
/// Each table keeps its ids unique on its own; nothing enforces it across tables.
pub fn duplicate_doc_ids(conn: &Connection) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for workspace in list_all_workspaces(conn)? {
        validate_doc_table_name(&workspace.doc_table)?;
        let mut stmt = conn
            .prepare(&format!("SELECT doc_id FROM {}", workspace.doc_table))
            .map_err(|e| e.to_string())?;
        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for id in ids {
            owners.entry(id.map_err(|e| e.to_string())?).or_default().push(workspace.id.clone());
        }
    }
    Ok(owners.into_iter().filter(|(_, workspaces)| workspaces.len() > 1).collect())
}
