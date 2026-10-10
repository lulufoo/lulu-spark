use std::fs;

use serde_json::json;

use crate::services::spark_read::resolve_note_for_open;
use crate::test_support::TestSandbox;

const ID: &str = "abcdefabcdefabcdefabcdefabcdefab";

fn seed_index(sandbox: &TestSandbox) {
    let notes = sandbox.data_dir().join("notes");
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
    fs::write(notes.join("index.json"), index.to_string()).expect("write index");
}

#[test]
fn resolve_returns_index_entry_for_known_id() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let out = resolve_note_for_open(sandbox.config_dir(), ID);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["id"], ID);
    assert_eq!(out["common_path"], "ai/note.md");
    assert_eq!(out["created_at"], "202606190004");
    assert_eq!(out["layers"], json!(["raw", "digest"]));
    assert_eq!(out["title"], "A note");
}

#[test]
fn resolve_trims_the_id() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let out = resolve_note_for_open(sandbox.config_dir(), &format!("  {ID}\n"));
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
        let out = resolve_note_for_open(sandbox.config_dir(), bad);
        assert_eq!(out["ok"], false, "id {bad:?}");
        assert_eq!(out["error"], "Invalid id");
    }
}

#[test]
fn resolve_reports_unknown_id() {
    let sandbox = TestSandbox::new();
    seed_index(&sandbox);
    let out = resolve_note_for_open(sandbox.config_dir(), "ffffffffffffffffffffffffffffffff");
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "Entry not found");
}

#[test]
fn resolve_reports_missing_index() {
    let sandbox = TestSandbox::new();
    let out = resolve_note_for_open(sandbox.config_dir(), ID);
    assert!(out.get("error").is_some(), "{out}");
    assert_ne!(out["ok"], true);
}
