//! Resolve a note id for the in-app open flow (`[title](note:<id>)` links in chat).

use serde_json::{json, Value};

use crate::services::spark_data::{self, entry_from_doc, find_doc};

/// Look up document `id` in any workspace. Success is its index entry plus `id` and `ok`.
/// Never returns a file body or an absolute path.
pub fn resolve_doc_for_open(id: &str) -> Value {
    let id = id.trim();
    if !spark_data::is_valid_doc_id(id) {
        return json!({ "id": id, "ok": false, "error": "Invalid id" });
    }
    let found = spark_data::open().and_then(|conn| {
        let Some(found) = find_doc(&conn, id)? else {
            return Ok(None);
        };
        let entry = entry_from_doc(&found.doc, &found.workspace.id)?;
        Ok(Some(entry))
    });
    let entry = match found {
        Ok(Some(entry)) => entry,
        Ok(None) => return json!({ "id": id, "ok": false, "error": "Entry not found" }),
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let mut resolved = entry.as_object().cloned().unwrap_or_default();
    resolved.insert("id".into(), json!(id));
    resolved.insert("ok".into(), json!(true));
    Value::Object(resolved)
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_open.rs"]
mod tests;
