use super::*;
use std::fs;

use crate::config::paths;
use crate::test_support::TestSandbox;

fn with_cache<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    f();
}

#[test]
fn key_md5_matches_sample() {
    assert_eq!(
        highlight_key_md5("doc:20260827/why-cache.md"),
        "42b1b5306365f7d39bccb5561940862d"
    );
}

#[test]
fn add_and_get_writes_under_doc_highlights_md5() {
    with_cache(|| {
        let key = "doc:20260827/why-cache.md";
        let added = update_doc_highlights(
            key,
            json!({ "text": "执行缓存", "occurrence": 0 }),
            "20260827T154000",
        );
        assert_eq!(added["ok"], true);
        assert!(added["id"].as_str().unwrap().len() >= 8);

        let cache = paths::cache_dir().expect("cache");
        let file = cache
            .join("doc-highlights")
            .join("42b1b5306365f7d39bccb5561940862d.json");
        assert!(file.is_file(), "expected {file:?}");
        let on_disk: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(on_disk["key"], key);
        assert_eq!(on_disk["highlights"][0]["text"], "执行缓存");

        let got = get_doc_highlights(key);
        assert_eq!(got["key"], key);
        assert_eq!(got["highlights"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn delete_last_highlight_removes_cache_file() {
    with_cache(|| {
        let key = "doc:default-0123456789abcdef0123456789abcdef";
        let added = update_doc_highlights(key, json!({ "text": "only" }), "t1");
        let id = added["id"].as_str().unwrap().to_string();
        let removed = update_doc_highlights(key, json!({ "id": id }), "t2");
        assert_eq!(removed["ok"], true);
        let got = get_doc_highlights(key);
        assert_eq!(got["highlights"].as_array().unwrap().len(), 0);
        let dir = paths::doc_highlights_dir().expect("dir");
        if dir.exists() {
            let leftover: Vec<_> = fs::read_dir(&dir).unwrap().collect();
            assert!(leftover.is_empty(), "empty cache should not keep json files");
        }
    });
}

#[test]
fn rejects_unknown_prefix_and_does_not_touch_annotation() {
    with_cache(|| {
        for key in ["inbox/note.md", "notes:20260827/a.md", "knowledge:a/b.md", "todos:task_1"] {
            assert_eq!(get_doc_highlights(key)["_status"], 400, "{key}");
            let written = update_doc_highlights(key, json!({ "text": "x", "occurrence": 0 }), "20260827T154000");
            assert_eq!(written["_status"], 400, "{key}");
        }
        let dir = paths::cache_dir().expect("cache");
        assert!(!dir.join("doc-highlights").join("inbox").exists());
    });
}
