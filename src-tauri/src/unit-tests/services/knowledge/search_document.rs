use super::*;
use serde_json::json;

use crate::test_support::TestSandbox;

#[test]
fn search_missing_q_is_400() {
    let sandbox = TestSandbox::new();
    let v = search_document_mcp(sandbox.config_dir(), "  ", None);
    assert_eq!(v["error"], "Missing q");
    assert_eq!(v["_status"], 400);
}

#[test]
fn merge_normalizes_notes_then_knowledge() {
    let notes = json!({
        "items": [{
            "note_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "catalog": "inbox",
            "title": "Note title",
            "matches": [{ "snippet": "note hit" }]
        }]
    });
    let knowledge = json!({
        "items": [{
            "id": "kb1",
            "title": "KB title",
            "snippet": "kb hit"
        }]
    });
    let v = merge_document_search(&notes, &knowledge);
    let items = v["items"].as_array().expect("items");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(items[0]["title"], "Note title");
    assert_eq!(items[0]["snippet"], "note hit");
    assert_eq!(items[0]["category"], "notes");
    assert!(items[0].get("catalog").is_none());
    assert_eq!(items[1]["id"], "kb1");
    assert_eq!(items[1]["category"], "knowledge");
    assert_eq!(items[1]["snippet"], "kb hit");
}

#[test]
fn merge_keeps_knowledge_when_notes_fail() {
    let notes = json!({ "error": "notes down", "_status": 503 });
    let knowledge = json!({
        "items": [{ "id": "kb1", "title": "KB", "snippet": "ok" }]
    });
    let v = merge_document_search(&notes, &knowledge);
    let items = v["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["category"], "knowledge");
    assert!(v.get("error").is_none());
}

#[test]
fn merge_returns_first_error_when_both_fail() {
    let notes = json!({ "error": "notes down", "_status": 503 });
    let knowledge = json!({ "error": "kb down", "_status": 503 });
    let v = merge_document_search(&notes, &knowledge);
    assert_eq!(v["error"], "notes down");
    assert_eq!(v["_status"], 503);
}
