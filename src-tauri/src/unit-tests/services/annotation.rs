use super::*;
use std::fs;

use crate::test_support::with_corpus;

#[test]
fn set_done_true_writes_done_key() {
    with_corpus(true, |dir, corpus| {
        let cp = "ai/note-done.md";
        let v = set_done(dir.path(), cp, true);
        assert_eq!(v["ok"], json!(true));
        let p = annotation_json_path(&corpus, cp).expect("path");
        let read: Value =
            serde_json::from_str(&fs::read_to_string(&p).expect("read")).expect("json");
        assert_eq!(read["done"], json!(true));
    });
}

#[test]
fn set_importance_high() {
    with_corpus(true, |dir, corpus| {
        let cp = "ai/note-importance.md";
        let v = set_importance(dir.path(), cp, Some("high".into()));
        assert_eq!(v["ok"], json!(true));
        let p = annotation_json_path(&corpus, cp).expect("path");
        let read: Value =
            serde_json::from_str(&fs::read_to_string(&p).expect("read")).expect("json");
        assert_eq!(read["importance"], json!("high"));
    });
}

#[test]
fn update_comments_new_returns_id() {
    with_corpus(true, |dir, _corpus| {
        let v = update_comments(
            dir.path(),
            "ai/note-comments.md",
            "raw",
            json!({ "text": "hi" }),
            String::new(),
        );
        assert_eq!(v["ok"], json!(true));
        assert_eq!(v["id"].as_str().map(|s| s.len()), Some(12));
    });
}

#[test]
fn update_links_invalid_url() {
    with_corpus(true, |dir, _corpus| {
        let v = update_links(
            dir.path(),
            "ai/note-bad-url.md",
            json!([{ "url": "ftp://bad" }]),
        );
        assert!(v["error"].as_str().unwrap().starts_with("Invalid url:"));
    });
}

#[test]
fn update_links_invalid_common_path() {
    with_corpus(true, |dir, _corpus| {
        let v = update_links(
            dir.path(),
            "../evil.md",
            json!([{ "url": "https://github.com/foo/bar" }]),
        );
        assert_eq!(v["error"], "Invalid common_path");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn update_links_persists_links_array() {
    with_corpus(true, |dir, corpus| {
        let cp = "ai/note-links.md";
        let links = json!([{ "url": "https://github.com/foo/bar" }]);
        let v = update_links(dir.path(), cp, links.clone());
        assert_eq!(v["ok"], json!(true));
        let p = annotation_json_path(&corpus, cp).expect("path");
        let read: Value =
            serde_json::from_str(&fs::read_to_string(&p).expect("read")).expect("json");
        assert_eq!(read["links"], links);
    });
}

#[test]
fn update_links_empty_array_removes_links_key() {
    with_corpus(true, |dir, corpus| {
        let cp = "ai/note-clear-links.md";
        let seed = update_links(
            dir.path(),
            cp,
            json!([{ "url": "https://github.com/foo/bar" }]),
        );
        assert_eq!(seed["ok"], json!(true));
        let v = update_links(dir.path(), cp, json!([]));
        assert_eq!(v["ok"], json!(true));
        let p = annotation_json_path(&corpus, cp).expect("path");
        assert!(!p.exists());
    });
}

#[test]
fn update_links_creates_annotation_when_topic_dir_missing() {
    with_corpus(false, |dir, corpus| {
        let cp = "inbox/new-note.md";
        let v = update_links(
            dir.path(),
            cp,
            json!([{ "url": "https://github.com/foo/bar" }]),
        );
        assert_eq!(v["ok"], json!(true));
        let p = annotation_json_path(&corpus, cp).expect("path");
        assert!(p.is_file());
    });
}

#[test]
fn reorder_comments_persists_new_order() {
    with_corpus(true, |dir, corpus| {
        let cp = "ai/note-reorder.md";
        let seed = update_comments(
            dir.path(),
            cp,
            "raw",
            json!({ "text": "first" }),
            String::new(),
        );
        assert_eq!(seed["ok"], json!(true));
        let id1 = seed["id"].as_str().expect("id1").to_string();
        let second = update_comments(
            dir.path(),
            cp,
            "raw",
            json!({ "text": "second" }),
            String::new(),
        );
        let id2 = second["id"].as_str().expect("id2").to_string();
        let third = update_comments(
            dir.path(),
            cp,
            "raw",
            json!({ "text": "third" }),
            String::new(),
        );
        let id3 = third["id"].as_str().expect("id3").to_string();

        let v = reorder_comments(
            dir.path(),
            cp,
            "raw",
            vec![id3.clone(), id1.clone(), id2.clone()],
        );
        assert_eq!(v["ok"], json!(true));

        let p = annotation_json_path(&corpus, cp).expect("path");
        let read: Value =
            serde_json::from_str(&fs::read_to_string(&p).expect("read")).expect("json");
        let ids: Vec<String> = read["raw"]["comments"]
            .as_array()
            .expect("comments")
            .iter()
            .filter_map(|c| c.get("id").and_then(|v| v.as_str()).map(String::from))
            .collect();
        assert_eq!(ids, vec![id3, id1, id2]);
    });
}

/// Pre-fix semantics: filter_map without len check — reproduces silent drop + ok.
#[test]
fn repro_legacy_reorder_drops_unknown_id_without_error() {
    let comments = vec![
        json!({ "id": "id_a", "text": "a" }),
        json!({ "id": "id_b", "text": "b" }),
    ];
    let id_map: Map<String, Value> = comments
        .into_iter()
        .filter_map(|c| {
            let id = c.get("id")?.as_str()?.to_string();
            Some((id, c))
        })
        .collect();
    let ids = vec!["id_b".to_string(), "ghost".to_string(), "id_a".to_string()];
    let reordered: Vec<Value> = ids
        .iter()
        .filter_map(|id| id_map.get(id).cloned())
        .collect();
    assert_eq!(reordered.len(), 2);
    assert_eq!(
        reordered[0].get("id").and_then(|v| v.as_str()),
        Some("id_b")
    );
    assert_eq!(reordered.len(), ids.len() - 1);
}

#[test]
fn reorder_comments_rejects_unknown_id() {
    with_corpus(true, |dir, _corpus| {
        let cp = "ai/note-reorder-bad.md";
        let seed = update_comments(
            dir.path(),
            cp,
            "raw",
            json!({ "text": "only" }),
            String::new(),
        );
        let id1 = seed["id"].as_str().expect("id").to_string();
        let v = reorder_comments(dir.path(), cp, "raw", vec![id1, "nope00000000".into()]);
        assert_eq!(v["error"], "Comment id not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn repro_reorder_invalid_common_path_returns_400() {
    with_corpus(true, |dir, _corpus| {
        let v = reorder_comments(dir.path(), "../evil.md", "raw", vec!["x".into()]);
        assert_eq!(v["error"], "Invalid common_path");
        assert_eq!(v["_status"], 400);
    });
}
