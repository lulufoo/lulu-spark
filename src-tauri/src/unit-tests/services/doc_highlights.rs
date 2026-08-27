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
        highlight_key_md5("notes:20260827/why-cache.md"),
        "7e175ab16c747d8769bcd2cfdd6abc32"
    );
}

#[test]
fn add_and_get_writes_under_doc_highlights_md5() {
    with_cache(|| {
        let key = "notes:20260827/why-cache.md";
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
            .join("7e175ab16c747d8769bcd2cfdd6abc32.json");
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
        let key = "knowledge:acme/docs/foo.md";
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
        let bad = get_doc_highlights("inbox/note.md");
        assert_eq!(bad["_status"], 400);
        let dir = paths::cache_dir().expect("cache");
        assert!(!dir.join("doc-highlights").join("inbox").exists());
    });
}
