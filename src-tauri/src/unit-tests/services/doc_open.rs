use std::fs;

use serde_json::json;

use crate::services::spark_read::resolve_doc_for_open;
use crate::test_support::{seed_note_entries, TestSandbox};

const ID: &str = "abcdefabcdefabcdefabcdefabcdefab";

fn seed_index(sandbox: &TestSandbox) {
    let notes = sandbox.data_dir().join("workspaces").join("default");
    fs::create_dir_all(&notes).expect("mkdir notes");
    let index = json!({
        "entries": {
            ID: {
                "common_path": "ai/note.md",
                "created_at": "202606190004",
                "layers": ["raw", "digest"],
                "title": "A note"
            }
        }
    });
    seed_note_entries(&index["entries"]);
}

#[test]
fn resolve_returns_index_entry_for_known_id() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let out = resolve_doc_for_open(ID);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["id"], ID);
    assert_eq!(out["folder"], "ai");
    assert!(out.get("common_path").is_none());
    assert_eq!(out["created_at"], "202606190004");
    assert_eq!(out["layers"], json!(["raw", "digest"]));
    assert_eq!(out["title"], "A note");
}

#[test]
fn resolve_trims_the_id() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let out = resolve_doc_for_open(&format!("  {ID}\n"));
    assert_eq!(out["ok"], true);
    assert_eq!(out["id"], ID);
}

#[test]
fn resolve_rejects_malformed_ids() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let bad_ids: Vec<String> = ["", "bad", "../../etc/passwd"]
        .iter()
        .map(|s| s.to_string())
        .chain([ "z".repeat(32), ID[..31].to_string() ])
        .collect();
    for bad in &bad_ids {
        let out = resolve_doc_for_open(bad);
        assert_eq!(out["ok"], false, "id {bad:?}");
        assert_eq!(out["error"], "Invalid id");
    }
}

#[test]
fn resolve_reports_unknown_id() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let out = resolve_doc_for_open("ffffffffffffffffffffffffffffffff");
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "Entry not found");
}

#[test]
fn resolve_reports_missing_index() {
    let _sandbox = TestSandbox::new();
    let out = resolve_doc_for_open(ID);
    assert!(out.get("error").is_some(), "{out}");
    assert_ne!(out["ok"], true);
}
