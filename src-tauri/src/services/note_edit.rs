//! Edit the document row identified by doc id, in one immediate transaction.
//!
//! The row is found in whichever workspace's table holds it.

use rusqlite::{Connection, TransactionBehavior};
use serde_json::{json, Value};

use crate::services::spark_data::{self, delete_unused_tags, find_doc, update_doc, DocRow, FoundDoc};

pub fn invalid_note_id() -> Value {
    json!({ "error": "Invalid id", "_status": 400 })
}

pub fn clean_note_id(id: &str) -> Result<&str, Value> {
    let id = id.trim();
    if id.len() != 32 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(invalid_note_id());
    }
    Ok(id)
}

fn internal(error: impl ToString) -> Value {
    json!({ "error": error.to_string(), "_status": 500 })
}

/// The document with this doc id and the workspace it lives in.
pub fn find_note(conn: &Connection, id: &str) -> Result<FoundDoc, Value> {
    find_doc(conn, id)
        .map_err(internal)?
        .ok_or_else(|| json!({ "error": "Entry not found", "_status": 404 }))
}

/// Run `edit` on the document `id`. `edit` returns the reply; an `Err` reply
/// leaves the row and everything else it touched unchanged. Tags no document references
/// any more are dropped in the same transaction.
pub fn edit_note(
    id: &str,
    edit: impl FnOnce(&Connection, &mut DocRow) -> Result<Value, Value>,
) -> Value {
    let result = (|| -> Result<Value, Value> {
        let id = clean_note_id(id)?;
        let mut conn = spark_data::open().map_err(internal)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(internal)?;
        let mut found = find_note(&tx, id)?;
        let reply = edit(&tx, &mut found.doc)?;
        update_doc(&tx, &found.workspace.doc_table, &found.doc).map_err(internal)?;
        delete_unused_tags(&tx).map_err(internal)?;
        tx.commit().map_err(internal)?;
        Ok(reply)
    })();
    result.unwrap_or_else(|reply| reply)
}
