use super::*;
use std::path::PathBuf;

#[test]
fn build_search_body_has_required_fields() {
    let body = build_search_body("rust", 10);
    assert_eq!(body["q"], "rust");
    assert_eq!(body["limit"], 10);
    assert_eq!(body["attributesToCrop"], json!(["body"]));
    assert_eq!(body["cropLength"], 80);
    assert_eq!(body["attributesToHighlight"], json!(["body"]));
}

#[test]
fn parse_limit_caps_at_50() {
    assert_eq!(parse_limit(None), 10);
    assert_eq!(parse_limit(Some(100)), 50);
}

#[test]
fn empty_q_returns_400_shape() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let backend = MeiliBackend::new(&root);
    let v = backend.search("knowledge", "  ", None);
    assert_eq!(v, json!({ "error": "q parameter required" }));
}

#[test]
fn map_unavailable_and_not_indexed() {
    assert_eq!(
        map_search_result(None),
        json!({ "hits": [], "error": "unavailable" })
    );
    assert_eq!(
        map_search_result(Some(json!({ "code": "index_not_found" }))),
        json!({ "hits": [], "error": "not_indexed" })
    );
}

#[test]
fn map_success_hits() {
    let v = map_search_result(Some(json!({
        "hits": [{ "id": "a", "title": "t" }]
    })));
    assert_eq!(v["hits"][0]["id"], "a");
    assert!(v.get("error").is_none());
}

#[test]
fn search_path_uses_index_uid() {
    assert_eq!(search_path("knowledge"), "/indexes/knowledge/search");
    assert_eq!(search_path("workbench"), "/indexes/workbench/search");
}

#[test]
fn build_knowledge_document_extracts_title() {
    let doc = build_knowledge_document("o/r", "docs/a.md", "# Hello\n\nbody", "desc");
    assert_eq!(doc["title"], "Hello");
    assert_eq!(doc["repo"], "o/r");
    assert!(doc["id"].as_str().unwrap().starts_with("r__docs__"));
}
