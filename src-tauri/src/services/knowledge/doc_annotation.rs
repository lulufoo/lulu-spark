//! Comments, links and highlights of one knowledge document, stored on its
//! `knowledge_doc` row. Highlights belong to the raw document and carry no layer.

use std::path::Path;

use rusqlite::TransactionBehavior;
use serde_json::{json, Map, Value};

use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::{kb_dir_name, kb_safe_path};
use crate::services::spark_data::{
    self, find_knowledge_doc_id, get_knowledge_doc, register_knowledge_doc, update_knowledge_doc,
    KnowledgeDocRow,
};

pub(super) fn err_status(msg: &str) -> u16 {
    if msg.contains("invalid") || msg.contains("traversal") {
        400
    } else if msg.contains("not cloned") || msg.contains("not found") {
        404
    } else {
        500
    }
}

fn error_value(msg: &str, status: u16) -> Value {
    json!({ "error": msg, "_status": status })
}

/// Read-modify-write one document's annotations. The file must exist; its record is
/// created on first use. `edit` returns the JSON reply, or an error reply that
/// leaves the row untouched.
pub(super) fn edit_doc(
    repo_root: &Path,
    repo: &str,
    path: &str,
    edit: impl FnOnce(&mut KnowledgeDocRow) -> Result<Value, Value>,
) -> Value {
    let kb_root = std::path::PathBuf::from(knowledge_root_string(repo_root));
    let target = match kb_safe_path(&kb_root, repo, path) {
        Ok(p) => p,
        Err(e) => return error_value(&e, err_status(&e)),
    };
    if !target.is_file() {
        return error_value(&format!("file not found: {}", path.trim()), 404);
    }
    let result = (|| -> Result<Value, Value> {
        let repo_dir = kb_dir_name(repo).map_err(|e| error_value(&e, 400))?;
        let mut conn = spark_data::open().map_err(|e| error_value(&e, 500))?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| error_value(&e.to_string(), 500))?;
        let doc_id = register_knowledge_doc(&tx, repo_dir, path.trim()).map_err(|e| error_value(&e, 500))?;
        let mut row = get_knowledge_doc(&tx, &doc_id)
            .map_err(|e| error_value(&e, 500))?
            .ok_or_else(|| error_value("knowledge record vanished", 500))?;
        let reply = edit(&mut row)?;
        update_knowledge_doc(&tx, &row).map_err(|e| error_value(&e, 500))?;
        tx.commit().map_err(|e| error_value(&e.to_string(), 500))?;
        Ok(reply)
    })();
    result.unwrap_or_else(|err| err)
}

/// Annotations in the shape the viewer already reads: only non-empty fields.
pub fn kb_annotation_json(repo_root: &Path, repo: &str, path: &str) -> Value {
    let kb_root = std::path::PathBuf::from(knowledge_root_string(repo_root));
    if let Err(e) = kb_safe_path(&kb_root, repo, path) {
        return error_value(&e, err_status(&e));
    }
    let lookup = || -> Result<Option<KnowledgeDocRow>, String> {
        let conn = spark_data::open()?;
        let repo_dir = kb_dir_name(repo)?;
        match find_knowledge_doc_id(&conn, repo_dir, path.trim())? {
            Some(id) => get_knowledge_doc(&conn, &id),
            None => Ok(None),
        }
    };
    let row = match lookup() {
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
