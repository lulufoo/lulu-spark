//! Resolve a knowledge `doc_id` for the in-app open flow (`[title](knowledge:<id>)`).

use serde_json::{json, Value};

use super::mcp::locate_knowledge_doc;

/// Look up knowledge document `id`. Success is `{id, ok, topic, path}`.
/// `path` is the tree location, not the document identity. Never returns a body
/// or an absolute path.
pub fn resolve_knowledge_for_open(id: &str) -> Value {
    match locate_knowledge_doc(id) {
        Ok(doc) => json!({
            "id": id.trim(),
            "ok": true,
            "topic": doc.repo,
            "path": doc.path,
        }),
        Err(reply) => reply,
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/open.rs"]
mod tests;
