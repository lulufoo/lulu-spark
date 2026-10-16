//! Edit the `note` row that sits at a `common_path`, in one immediate transaction.

use rusqlite::{Connection, TransactionBehavior};
use serde_json::{json, Value};

use crate::services::spark_data::{
    self, delete_unused_tags, find_note_ids_by_path, get_note, update_note, NoteRow,
};

pub fn invalid_common_path() -> Value {
    json!({ "error": "Invalid common_path", "_status": 400 })
}

pub fn clean_common_path(common_path: &str) -> Result<&str, Value> {
    let cp = common_path.trim();
    if cp.is_empty() || cp.contains("..") || cp.starts_with('/') {
        return Err(invalid_common_path());
    }
    Ok(cp)
}

fn internal(error: impl ToString) -> Value {
    json!({ "error": error.to_string(), "_status": 500 })
}

/// The one note at `common_path`. Several notes can share a path in old data; refusing
/// beats annotating the wrong one.
pub fn find_note(conn: &Connection, common_path: &str) -> Result<NoteRow, Value> {
    let ids = find_note_ids_by_path(conn, common_path).map_err(internal)?;
    let id = match ids.as_slice() {
        [id] => id,
        [] => return Err(json!({ "error": "Entry not found", "_status": 404 })),
        _ => return Err(json!({ "error": "More than one entry has this path", "_status": 409 })),
    };
    get_note(conn, id)
        .map_err(internal)?
        .ok_or_else(|| json!({ "error": "Entry not found", "_status": 404 }))
}

/// Run `edit` on the note at `common_path`. `edit` returns the reply; an `Err` reply
/// leaves the row and everything else it touched unchanged. Tags no note references
/// any more are dropped in the same transaction.
pub fn edit_note(
    common_path: &str,
    edit: impl FnOnce(&Connection, &mut NoteRow) -> Result<Value, Value>,
) -> Value {
    let result = (|| -> Result<Value, Value> {
        let cp = clean_common_path(common_path)?;
        let mut conn = spark_data::open().map_err(internal)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(internal)?;
        let mut note = find_note(&tx, cp)?;
        let reply = edit(&tx, &mut note)?;
        update_note(&tx, &note).map_err(internal)?;
        delete_unused_tags(&tx).map_err(internal)?;
        tx.commit().map_err(internal)?;
        Ok(reply)
    })();
    result.unwrap_or_else(|reply| reply)
}
