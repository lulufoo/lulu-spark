use std::fs;

use serde_json::json;

use crate::services::notes::get_note_file;
use crate::test_support::TestSandbox;

const ID: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn seed_note(sandbox: &TestSandbox, layers: &[&str]) {
    let notes = sandbox.data_dir().join("notes");
    fs::create_dir_all(notes.join("raw/ai")).expect("mkdir raw");
    if layers.contains(&"raw") {
        fs::write(notes.join("raw/ai/note.md"), b"# raw body").expect("write raw");
    }
    let index = json!({
        "entries": { ID: { "common_path": "ai/note.md", "created_at": "202606190004", "layers": layers } }
    });
    fs::write(notes.join("index.json"), index.to_string()).expect("write index");
}

#[test]
fn get_note_file_copies_raw_into_dest_dir() {
    let sandbox = TestSandbox::new();
    seed_note(&sandbox, &["raw"]);
    let dest = sandbox.cache_dir().join("ws/s1");
    let out = get_note_file(sandbox.config_dir(), ID, dest.to_str().unwrap());
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["id"], ID);
    let path = out["path"].as_str().expect("path");
    assert!(path.ends_with(&format!("{ID}.md")));
    assert_eq!(fs::read_to_string(path).unwrap(), "# raw body");
    assert!(out.get("content").is_none(), "no body in the result");
}

#[test]
fn get_note_file_bumps_version_instead_of_overwriting() {
    let sandbox = TestSandbox::new();
    seed_note(&sandbox, &["raw"]);
    let dest = sandbox.cache_dir().join("ws/s2");
    let dest = dest.to_str().unwrap();
    let first = get_note_file(sandbox.config_dir(), ID, dest);
    let second = get_note_file(sandbox.config_dir(), ID, dest);
    assert!(second["path"].as_str().unwrap().ends_with(&format!("{ID}-v1.md")));
    assert_ne!(first["path"], second["path"]);
}

#[test]
fn get_note_file_errors_without_raw_layer_or_unknown_id() {
    let sandbox = TestSandbox::new();
    seed_note(&sandbox, &["digest"]);
    let dest = sandbox.cache_dir().join("ws/s3");
    let dest = dest.to_str().unwrap();
    let no_raw = get_note_file(sandbox.config_dir(), ID, dest);
    assert_eq!(no_raw["ok"], false);
    assert!(no_raw["error"].is_string(), "{no_raw}");
    let unknown = get_note_file(sandbox.config_dir(), "ffffffffffffffffffffffffffffffff", dest);
    assert_eq!(unknown["ok"], false);
    let bad = get_note_file(sandbox.config_dir(), "bad", dest);
    assert_eq!(bad["ok"], false);
    assert!(!sandbox.cache_dir().join("ws/s3").exists(), "failed lookups create no dir");
}

#[test]
fn get_note_file_rejects_dest_dir_inside_data() {
    let sandbox = TestSandbox::new();
    seed_note(&sandbox, &["raw"]);
    let dest = sandbox.data_dir().join("notes/raw/ai");
    let out = get_note_file(sandbox.config_dir(), ID, dest.to_str().unwrap());
    assert_eq!(out["ok"], false);
    assert_eq!(out["_status"], 403);
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 1, "original raw dir untouched");
}
