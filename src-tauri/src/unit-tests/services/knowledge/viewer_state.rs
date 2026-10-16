use super::*;
use crate::test_support::TestSandbox;
use std::fs;

#[test]
fn get_missing_file_returns_empty_and_does_not_write() {
    let sandbox = TestSandbox::new();
    let value = get_json().expect("get");
    assert_eq!(value["id"], "");
    let path = sandbox.cache_dir().join("knowledge-viewer-state.json");
    assert!(!path.exists(), "missing file must not be seeded");
}

#[test]
fn set_then_get_round_trips_id() {
    let sandbox = TestSandbox::new();
    set_json("abcdefabcdefabcdefabcdefabcdefab").expect("set");
    let value = get_json().expect("get");
    assert_eq!(value["id"], "abcdefabcdefabcdefabcdefabcdefab");
    let path = sandbox.cache_dir().join("knowledge-viewer-state.json");
    assert!(path.is_file(), "set must write cache file");
}

#[test]
fn set_trims_id() {
    let _sandbox = TestSandbox::new();
    set_json("  abc  ").expect("set");
    let value = get_json().expect("get");
    assert_eq!(value["id"], "abc");
}

#[test]
fn corrupt_file_returns_empty_without_overwrite() {
    let sandbox = TestSandbox::new();
    let path = sandbox.cache_dir().join("knowledge-viewer-state.json");
    fs::write(&path, "{not-json").expect("write corrupt");
    let value = get_json().expect("get");
    assert_eq!(value["id"], "");
    assert_eq!(fs::read_to_string(&path).expect("read"), "{not-json");
}
