//! Unified document search: notes + knowledge, knowledge-shaped items.

use std::path::Path;

use serde_json::{json, Value};

use crate::services::spark_read::search_notes;

use super::mcp::search_knowledge_mcp;

const DOCUMENT_SEARCH_DEFAULT_LIMIT: u32 = 10;
const DOCUMENT_SEARCH_MAX_LIMIT: u32 = 50;

fn document_search_limit(limit: Option<u32>) -> u32 {
    match limit {
        None | Some(0) => DOCUMENT_SEARCH_DEFAULT_LIMIT,
        Some(n) => n.min(DOCUMENT_SEARCH_MAX_LIMIT),
    }
}

fn is_search_error(value: &Value) -> bool {
    value.get("error").and_then(|v| v.as_str()).is_some()
}

fn notes_item_to_document(item: &Value) -> Option<Value> {
    let id = item.get("note_id").and_then(|v| v.as_str())?;
    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let snippet = item
        .get("matches")
        .and_then(|v| v.as_array())
        .and_then(|matches| matches.first())
        .and_then(|m| m.get("snippet"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Some(json!({
        "id": id,
        "title": title,
        "snippet": snippet,
        "category": "notes",
    }))
}

fn knowledge_item_to_document(item: &Value) -> Option<Value> {
    let id = item.get("id").and_then(|v| v.as_str())?;
    Some(json!({
        "id": id,
        "title": item.get("title").and_then(|v| v.as_str()).unwrap_or(""),
        "snippet": item.get("snippet").and_then(|v| v.as_str()).unwrap_or(""),
        "category": "knowledge",
    }))
}

/// Merge notes + knowledge search payloads. One-sided failure keeps the other side.
pub fn merge_document_search(notes: &Value, knowledge: &Value) -> Value {
    let notes_err = is_search_error(notes);
    let knowledge_err = is_search_error(knowledge);
    if notes_err && knowledge_err {
        return notes.clone();
    }
    let mut items = Vec::new();
    if !notes_err {
        if let Some(arr) = notes.get("items").and_then(|v| v.as_array()) {
            items.extend(arr.iter().filter_map(notes_item_to_document));
        }
    }
    if !knowledge_err {
        if let Some(arr) = knowledge.get("items").and_then(|v| v.as_array()) {
            items.extend(arr.iter().filter_map(knowledge_item_to_document));
        }
    }
    json!({ "items": items })
}

/// Search notes and knowledge, then return a unified document list.
pub fn search_document_mcp(repo_root: &Path, q: &str, limit: Option<u32>) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "Missing q", "_status": 400 });
    }
    let limit = Some(document_search_limit(limit));
    let notes = search_notes(repo_root, q, None, limit);
    let knowledge = search_knowledge_mcp(repo_root, q, limit);
    merge_document_search(&notes, &knowledge)
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/search_document.rs"]
mod tests;
