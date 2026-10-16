//! Knowledge registry rules on `knowledge_doc`: register, move, remove, reconcile.
//!
//! Database only; the caller has already touched (or looked at) the files.
//! `directory_id` is the topic row; `path` is relative to that folder with `/`
//! separators.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension};

use crate::services::id::random_entry_id;

use super::knowledge_doc_row::{insert_knowledge_doc, KnowledgeDocRow};

/// Return the `doc_id` of `(directory_id, path)`, creating the record when there is
/// none and bringing a `missing` record back to `active`.
pub fn register_knowledge_doc(
    conn: &Connection,
    directory_id: &str,
    path: &str,
) -> Result<String, String> {
    let found: Option<String> = conn
        .query_row(
            "SELECT doc_id FROM knowledge_doc WHERE directory_id = ?1 AND path = ?2",
            params![directory_id, path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(doc_id) = found {
        conn.execute(
            "UPDATE knowledge_doc SET status = 'active' WHERE doc_id = ?1 AND status <> 'active'",
            params![doc_id],
        )
        .map_err(|e| e.to_string())?;
        return Ok(doc_id);
    }
    let doc_id = random_entry_id();
    insert_knowledge_doc(conn, &KnowledgeDocRow::new(&doc_id, directory_id, path))?;
    Ok(doc_id)
}

/// `path = ?` or anything below it. `substr` instead of `LIKE` so `%` and `_` in
/// names need no escaping.
const AT_OR_BELOW: &str = "(path = ?2 OR substr(path, 1, length(?2) + 1) = ?2 || '/')";

/// Delete the record at `path` and, for a folder, every record below it.
pub fn remove_knowledge_paths(
    conn: &Connection,
    directory_id: &str,
    path: &str,
) -> Result<usize, String> {
    conn.execute(
        &format!("DELETE FROM knowledge_doc WHERE directory_id = ?1 AND {AT_OR_BELOW}"),
        params![directory_id, path],
    )
    .map_err(|e| e.to_string())
}

/// Re-path the record at `from` (and, for a folder, everything below it) to `to`; the
/// `doc_id` and the annotations stay. The files were already moved, so the destination
/// is free on disk and any record still sitting there is stale: it is dropped first.
pub fn move_knowledge_paths(
    conn: &Connection,
    directory_id: &str,
    from: &str,
    to: &str,
) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    remove_knowledge_paths(&tx, directory_id, to)?;
    tx.execute(
        &format!(
            "UPDATE knowledge_doc SET path = ?3 || substr(path, length(?2) + 1)
             WHERE directory_id = ?1 AND {AT_OR_BELOW}"
        ),
        params![directory_id, from, to],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileStats {
    pub registered: usize,
    pub restored: usize,
    pub marked_missing: usize,
}

/// Bring one directory's records in line with what a scan found.
///
/// * `found`: every document path the scan walked.
/// * `file_exists`: direct check for a record the walk did not return (it may be a
///   file the walk deliberately skips); only a real absence counts as missing.
/// * `is_hidden`: paths under a hide rule. They were not walked on purpose, so their
///   records are never marked missing.
///
/// Nothing is ever deleted here; a vanished file keeps its record and comments.
pub fn reconcile_knowledge_directory(
    conn: &Connection,
    directory_id: &str,
    found: &HashSet<String>,
    file_exists: impl Fn(&str) -> bool,
    is_hidden: impl Fn(&str) -> bool,
) -> Result<ReconcileStats, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut stmt = tx
        .prepare("SELECT path, status FROM knowledge_doc WHERE directory_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![directory_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    let known: HashMap<String, String> = rows.into_iter().collect();

    let mut stats = ReconcileStats::default();
    for path in found {
        match known.get(path).map(String::as_str) {
            None => {
                register_knowledge_doc(&tx, directory_id, path)?;
                stats.registered += 1;
            }
            Some("missing") => {
                register_knowledge_doc(&tx, directory_id, path)?;
                stats.restored += 1;
            }
            Some(_) => {}
        }
    }
    for (path, status) in &known {
        if found.contains(path) {
            continue;
        }
        let present = file_exists(path);
        if present && status == "missing" {
            register_knowledge_doc(&tx, directory_id, path)?;
            stats.restored += 1;
        } else if !present && status == "active" && !is_hidden(path) {
            tx.execute(
                "UPDATE knowledge_doc SET status = 'missing' WHERE directory_id = ?1 AND path = ?2",
                params![directory_id, path],
            )
            .map_err(|e| e.to_string())?;
            stats.marked_missing += 1;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(stats)
}
