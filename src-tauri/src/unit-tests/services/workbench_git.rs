use super::*;
use crate::test_support::TestSandbox;
use std::fs;

fn exec_ok(repo: &std::path::Path, args: &[&str]) {
    let out = git::exec(repo, args).expect("git exec");
    assert!(out.success, "git {:?} failed: {}", args, out.stderr);
}

/// Sets up a temp git repo with a committed `f.md` ("original"), then calls `f(&wb)`.
fn with_committed_workbench<F: FnOnce(&std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    fs::create_dir_all(&wb).expect("mkdir");
    exec_ok(&wb, &["init"]);
    exec_ok(&wb, &["config", "user.email", "t@t.com"]);
    exec_ok(&wb, &["config", "user.name", "t"]);
    fs::write(wb.join("f.md"), "original").expect("write f.md");
    exec_ok(&wb, &["add", "f.md"]);
    exec_ok(&wb, &["commit", "-m", "base"]);
    f(wb.as_path());
}

fn with_plain_workbench<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    f();
}

#[test]
fn workbench_commit_errors_when_not_git() {
    with_plain_workbench(|| {
        let v = workbench_git_commit(&json!({}));
        assert!(v.get("error").is_some());
    });
}

#[test]
fn workbench_revert_errors_when_not_git() {
    with_plain_workbench(|| {
        let v = workbench_git_revert(&json!({ "path": "f.md", "type": "modified" }));
        assert!(v.get("error").is_some(), "expected error, got: {v}");
    });
}

#[test]
fn workbench_revert_modified_file_restores_content() {
    with_committed_workbench(|wb| {
        fs::write(wb.join("f.md"), "modified").expect("modify");
        let v = workbench_git_revert(&json!({ "path": "f.md", "type": "modified" }));
        assert_eq!(v["ok"], json!(true), "expected ok, got: {v}");
        let content = fs::read_to_string(wb.join("f.md")).expect("read");
        assert_eq!(content, "original", "file not restored");
    });
}

#[test]
fn workbench_revert_new_file_deletes_it() {
    with_committed_workbench(|wb| {
        let new_file = wb.join("new.md");
        fs::write(&new_file, "new content").expect("write new file");
        assert!(new_file.exists(), "pre-condition: file exists");
        let v = workbench_git_revert(&json!({ "path": "new.md", "type": "new" }));
        assert_eq!(v["ok"], json!(true), "expected ok, got: {v}");
        assert!(!new_file.exists(), "new file should be deleted after revert");
    });
}

#[test]
fn workbench_revert_empty_path_reverts_all() {
    with_committed_workbench(|wb| {
        fs::write(wb.join("f.md"), "modified").expect("modify");
        let v = workbench_git_revert(&json!({ "path": "", "type": "" }));
        assert_eq!(v["ok"], json!(true), "expected ok, got: {v}");
        let content = fs::read_to_string(wb.join("f.md")).expect("read");
        assert_eq!(content, "original", "revert-all did not restore file");
    });
}

#[test]
fn workbench_revert_deleted_type_uses_checkout() {
    with_committed_workbench(|wb| {
        fs::write(wb.join("f.md"), "changed").expect("modify for deleted-type test");
        let v = workbench_git_revert(&json!({ "path": "f.md", "type": "deleted" }));
        assert_eq!(v["ok"], json!(true), "expected ok for deleted type, got: {v}");
        let content = fs::read_to_string(wb.join("f.md")).expect("read");
        assert_eq!(content, "original", "file not restored for type=deleted");
    });
}
