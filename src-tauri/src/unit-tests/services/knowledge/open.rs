use std::fs;

use serde_json::json;

use crate::config::paths;
use crate::services::knowledge::{doc_map, resolve_knowledge_for_open};
use crate::test_support::TestSandbox;

fn plant(sandbox: &TestSandbox, rel: &str) -> std::path::PathBuf {
    let file = sandbox.knowledge_root().join(rel);
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, "# doc").expect("write");
    file
}

#[test]
fn resolves_id_to_repo_and_relative_path_without_absolute_path() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/guide/intro.md");
    let id = doc_map::remember_path(&file).expect("remember");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["id"], id);
    assert_eq!(out["repo"], "demo");
    assert_eq!(out["path"], "guide/intro.md");
    let text = out.to_string();
    assert!(
        !text.contains(sandbox.cache_dir().to_string_lossy().as_ref()),
        "must not leak an absolute path: {text}"
    );
    assert!(!text.contains("# doc"), "must not return the body: {text}");
}

#[test]
fn resolves_a_legacy_twelve_char_id() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/old.md");
    let canon = file.canonicalize().expect("canon");
    let map = paths::knowledge_doc_map_path().expect("map path");
    fs::create_dir_all(map.parent().unwrap()).expect("mkdir");
    fs::write(
        &map,
        json!({"version": 1, "entries": [{"id": "0123456789ab", "path": canon.to_string_lossy()}]})
            .to_string(),
    )
    .expect("legacy map");
    let out = resolve_knowledge_for_open("0123456789ab");
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["repo"], "demo");
    assert_eq!(out["path"], "old.md");
}

#[test]
fn unknown_id_is_an_error() {
    let _sandbox = TestSandbox::new();
    let out = resolve_knowledge_for_open("ffffffffffffffffffffffffffffffff");
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "Unknown document id");
}

#[test]
fn file_outside_the_knowledge_library_is_rejected() {
    let sandbox = TestSandbox::new();
    let outside = sandbox.cache_dir().join("elsewhere").join("x.md");
    fs::create_dir_all(outside.parent().unwrap()).expect("mkdir");
    fs::write(&outside, "x").expect("write");
    let id = doc_map::remember_path(&outside).expect("remember");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(out["error"], "Not in the knowledge library");
}

#[test]
fn file_directly_under_the_library_root_has_no_repo_and_is_rejected() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "loose.md");
    let id = doc_map::remember_path(&file).expect("remember");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(out["error"], "Not in the knowledge library");
}

#[test]
fn deleted_file_is_an_error() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/gone.md");
    let id = doc_map::remember_path(&file).expect("remember");
    fs::remove_file(&file).expect("remove");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(out["error"], "File not found");
}
