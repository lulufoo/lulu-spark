use super::*;
use std::fs;
use std::sync::Barrier;
use std::thread;

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

#[test]
fn saves_and_clears_one_comment_on_a_legacy_highlight() {
    with_cache(|| {
        let key = "doc:legacy";
        let path = cache_path(key).unwrap();
        let legacy = json!({ "key": key, "highlights": [
            { "id": "legacy-id", "text": "selected text", "occurrence": 2, "ts": "old" }
        ] });
        write_file(&path, &legacy).unwrap();

        let got = get_doc_highlights(key);
        assert_eq!(got["highlights"][0]["comments"], json!([]));
        let stored: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(stored, legacy);

        let created = set_doc_highlight_comment(key, "legacy-id", " first comment ");
        assert_eq!(created["ok"], true);
        let id = created["comments"][0]["id"].as_str().unwrap();
        assert!(!id.is_empty());
        assert_eq!(created["comments"], json!([{ "id": id, "text": "first comment" }]));

        let edited = set_doc_highlight_comment(key, "legacy-id", "updated comment");
        assert_eq!(edited["comments"], json!([{ "id": id, "text": "updated comment" }]));
        let got = get_doc_highlights(key);
        assert_eq!(got["highlights"][0], json!({
            "id": "legacy-id", "text": "selected text", "occurrence": 2, "ts": "old",
            "comments": [{ "id": id, "text": "updated comment" }]
        }));

        let cleared = set_doc_highlight_comment(key, "legacy-id", " \n ");
        assert_eq!(cleared, json!({ "ok": true, "comments": [] }));
        let got = get_doc_highlights(key);
        assert_eq!(got["highlights"][0]["id"], "legacy-id");
        assert_eq!(got["highlights"][0]["comments"], json!([]));
    });
}

#[test]
fn refuses_a_comment_save_after_removing_its_highlight() {
    with_cache(|| {
        let key = "doc:removed";
        let added = update_doc_highlights(key, json!({ "text": "selection" }), "t1");
        let id = added["id"].as_str().unwrap();
        assert_eq!(set_doc_highlight_comment(key, id, "comment")["ok"], true);
        assert_eq!(
            update_doc_highlights(key, json!({ "id": id }), "t2"),
            json!({ "ok": false, "requires_confirmation": true })
        );
        assert_eq!(get_doc_highlights(key)["highlights"][0]["comments"][0]["text"], "comment");
        assert_eq!(
            update_doc_highlights(key, json!({ "id": id, "confirm_comment_removal": true }), "t2")["ok"],
            true
        );
        assert_eq!(set_doc_highlight_comment(key, id, "late save")["_status"], 404);
        assert!(!cache_path(key).unwrap().exists());
    });
}

#[test]
fn serializes_different_writes_to_one_document() {
    with_cache(|| {
        let key = "doc:shared";
        let first = update_doc_highlights(key, json!({ "text": "first" }), "t1");
        let first_id = first["id"].as_str().unwrap();
        let second = update_doc_highlights(key, json!({ "text": "second" }), "t1");
        let second_id = second["id"].as_str().unwrap();
        let third = update_doc_highlights(key, json!({ "text": "remove this" }), "t1");
        let third_id = third["id"].as_str().unwrap();
        let start = Barrier::new(4);

        let replies = thread::scope(|scope| {
            [
                scope.spawn(|| {
                    start.wait();
                    set_doc_highlight_comment(key, first_id, "one")
                }),
                scope.spawn(|| {
                    start.wait();
                    set_doc_highlight_comment(key, second_id, "two")
                }),
                scope.spawn(|| {
                    start.wait();
                    update_doc_highlights(key, json!({ "id": third_id }), "t2")
                }),
                scope.spawn(|| {
                    start.wait();
                    update_doc_highlights(key, json!({ "text": "new selection" }), "t2")
                }),
            ]
            .map(|handle| handle.join().unwrap())
        });
        assert!(replies.iter().all(|reply| reply["ok"] == true), "{replies:?}");
        let got = get_doc_highlights(key);
        let highlights = got["highlights"].as_array().unwrap();
        assert_eq!(highlights.len(), 3);
        let find = |id: &str| highlights.iter().find(|row| row["id"] == id);
        assert_eq!(find(first_id).unwrap()["comments"][0]["text"], "one");
        assert_eq!(find(second_id).unwrap()["comments"][0]["text"], "two");
        assert!(find(third_id).is_none());
        assert!(find(replies[3]["id"].as_str().unwrap()).is_some());
    });
}
