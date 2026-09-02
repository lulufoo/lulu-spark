use super::*;
use std::fs;

use crate::test_support::TestSandbox;

#[test]
fn read_abs_file_returns_content() {
    let sandbox = TestSandbox::new();
    let file = sandbox.workbench_root().join("todo_tasks").join("note.md");
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, "hello").expect("write");
    let v = read_abs_file(&file.to_string_lossy());
    assert_eq!(v["content"], "hello", "{v}");
}

#[test]
fn write_abs_file_overwrites() {
    let sandbox = TestSandbox::new();
    let file = sandbox.knowledge_root().join("demo").join("doc.md");
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, "old").expect("write");
    let v = write_abs_file(&file.to_string_lossy(), "new");
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(fs::read_to_string(&file).expect("read"), "new");
}

#[test]
fn rejects_relative_and_missing() {
    let _sandbox = TestSandbox::new();
    let rel = read_abs_file("relative.md");
    assert_eq!(rel["_status"], 400, "{rel}");
    let missing = read_abs_file("/tmp/lulu-abs-file-missing-does-not-exist.md");
    assert_eq!(missing["_status"], 404, "{missing}");
}
