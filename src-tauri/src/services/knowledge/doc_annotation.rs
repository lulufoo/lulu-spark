//! Comments, links and highlights of one knowledge document, stored on its
//! `knowledge_doc` row. Highlights belong to the raw document and carry no layer.

use rusqlite::TransactionBehavior;
use serde_json::{json, Map, Value};

use crate::services::spark_data::{self, get_knowledge_doc, update_knowledge_doc, KnowledgeDocRow};

use super::mcp::locate_knowledge_doc;

fn error_value(msg: &str, status: u16) -> Value {
    json!({ "error": msg, "_status": status })
}

/// Read-modify-write one document's annotations. The file must exist.
pub(super) fn edit_doc(
    id: &str,
    edit: impl FnOnce(&mut KnowledgeDocRow) -> Result<Value, Value>,
) -> Value {
    if let Err(reply) = locate_knowledge_doc(id) {
        return reply;
    }
    let result = (|| -> Result<Value, Value> {
        let mut conn = spark_data::open().map_err(|e| error_value(&e, 500))?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| error_value(&e.to_string(), 500))?;
        let mut row = get_knowledge_doc(&tx, id.trim())
            .map_err(|e| error_value(&e, 500))?
            .ok_or_else(|| error_value("Unknown document id", 404))?;
        let reply = edit(&mut row)?;
        update_knowledge_doc(&tx, &row).map_err(|e| error_value(&e, 500))?;
        tx.commit().map_err(|e| error_value(&e.to_string(), 500))?;
        Ok(reply)
    })();
    result.unwrap_or_else(|err| err)
}

/// Annotations in the shape the viewer already reads: only non-empty fields.
pub fn kb_annotation_json(id: &str) -> Value {
    let id = id.trim();
    if id.is_empty() {
        return error_value("Missing id", 400);
    }
    let row = match spark_data::open().and_then(|conn| get_knowledge_doc(&conn, id)) {
        Ok(row) => row,
        Err(e) => return error_value(&e, 500),
    };
    let mut out = Map::new();
    if let Some(row) = row {
        for (key, value) in [
            ("comments", row.comments),
            ("links", row.links),
            ("highlights", row.highlights),
        ] {
            if value.as_array().is_some_and(|a| !a.is_empty()) {
                out.insert(key.into(), value);
            }
        }
    }
    Value::Object(out)
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/doc_annotation.rs"]
mod tests;
