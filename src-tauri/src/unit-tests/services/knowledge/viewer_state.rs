use super::*;
use crate::test_support::TestSandbox;
use std::fs;

#[test]
fn get_missing_file_returns_empty_and_does_not_write() {
    let sandbox = TestSandbox::new();
    let value = get_json().expect("get");
    assert_eq!(value["repo"], "");
    assert_eq!(value["path"], "");
    let path = sandbox.cache_dir().join("knowledge-viewer-state.json");
    assert!(!path.exists(), "missing file must not be seeded");
}

#[test]
fn set_then_get_round_trips_repo_and_md_path() {
    let sandbox = TestSandbox::new();
    set_json("owner/repo", "docs/guide.md").expect("set");
    let value = get_json().expect("get");
    assert_eq!(value["repo"], "owner/repo");
    assert_eq!(value["path"], "docs/guide.md");
    let path = sandbox.cache_dir().join("knowledge-viewer-state.json");
    assert!(path.is_file(), "set must write cache file");
}

#[test]
fn set_drops_non_md_path() {
    let _sandbox = TestSandbox::new();
    set_json("owner/repo", "notes.txt").expect("set");
    let value = get_json().expect("get");
    assert_eq!(value["repo"], "owner/repo");
    assert_eq!(value["path"], "");
}

#[test]
fn corrupt_file_returns_empty_without_overwrite() {
    let sandbox = TestSandbox::new();
    let path = sandbox.cache_dir().join("knowledge-viewer-state.json");
    fs::write(&path, "{not-json").expect("write corrupt");
    let value = get_json().expect("get");
    assert_eq!(value["repo"], "");
    assert_eq!(value["path"], "");
    assert_eq!(fs::read_to_string(&path).expect("read"), "{not-json");
}
