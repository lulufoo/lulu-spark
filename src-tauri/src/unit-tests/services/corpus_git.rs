use super::*;
use crate::config::settings;
use std::fs;

fn exec_ok(repo: &std::path::Path, args: &[&str]) {
    let out = git::exec(repo, args).expect("git exec");
    assert!(out.success, "git {:?} failed: {}", args, out.stderr);
}

fn with_git_corpus<F: FnOnce(&std::path::Path)>(f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    git::exec(&corpus, &["init"]).expect("init");
    git::exec(&corpus, &["config", "user.email", "t@t.com"]).expect("e");
    git::exec(&corpus, &["config", "user.name", "t"]).expect("n");
    fs::write(corpus.join("f.md"), "a").expect("w");
    settings::write_test_config(dir.path(), &corpus, None);
    f(dir.path());
    settings::set_test_config_dir(None);
}

/// Sets up a temp git repo with a committed `f.md` ("original"), then calls `f(&corpus)`.
fn with_committed_corpus<F: FnOnce(&std::path::Path)>(f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    exec_ok(&corpus, &["init"]);
    exec_ok(&corpus, &["config", "user.email", "t@t.com"]);
    exec_ok(&corpus, &["config", "user.name", "t"]);
    fs::write(corpus.join("f.md"), "original").expect("write f.md");
    exec_ok(&corpus, &["add", "f.md"]);
    exec_ok(&corpus, &["commit", "-m", "base"]);
    settings::write_test_config(dir.path(), &corpus, None);
    f(&corpus);
    settings::set_test_config_dir(None);
}

// ── corpus_git_commit ─────────────────────────────────────────────────────────

#[test]
fn corpus_commit_errors_when_corpus_not_git() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    settings::write_test_config(dir.path(), &corpus, None);
    let v = corpus_git_commit(&json!({}));
    assert!(v.get("error").is_some());
    settings::set_test_config_dir(None);
}

// ── corpus_git_revert ─────────────────────────────────────────────────────────

#[test]
fn corpus_revert_errors_when_corpus_not_git() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    settings::write_test_config(dir.path(), &corpus, None);
    let v = corpus_git_revert(&json!({ "path": "f.md", "type": "modified" }));
    assert!(v.get("error").is_some(), "expected error, got: {v}");
    settings::set_test_config_dir(None);
}

#[test]
fn corpus_revert_modified_file_restores_content() {
    with_committed_corpus(|corpus| {
        fs::write(corpus.join("f.md"), "modified").expect("modify");
        let v = corpus_git_revert(&json!({ "path": "f.md", "type": "modified" }));
        assert_eq!(v["ok"], json!(true), "expected ok, got: {v}");
        let content = fs::read_to_string(corpus.join("f.md")).expect("read");
        assert_eq!(content, "original", "file not restored");
    });
}

#[test]
fn corpus_revert_new_file_deletes_it() {
    with_committed_corpus(|corpus| {
        let new_file = corpus.join("new.md");
        fs::write(&new_file, "new content").expect("write new file");
        assert!(new_file.exists(), "pre-condition: file exists");
        let v = corpus_git_revert(&json!({ "path": "new.md", "type": "new" }));
        assert_eq!(v["ok"], json!(true), "expected ok, got: {v}");
        assert!(!new_file.exists(), "new file should be deleted after revert");
    });
}

#[test]
fn corpus_revert_empty_path_reverts_all() {
    with_committed_corpus(|corpus| {
        fs::write(corpus.join("f.md"), "modified").expect("modify");
        let v = corpus_git_revert(&json!({ "path": "", "type": "" }));
        assert_eq!(v["ok"], json!(true), "expected ok, got: {v}");
        let content = fs::read_to_string(corpus.join("f.md")).expect("read");
        assert_eq!(content, "original", "revert-all did not restore file");
    });
}

#[test]
fn corpus_revert_deleted_type_uses_checkout() {
    with_committed_corpus(|corpus| {
        fs::write(corpus.join("f.md"), "changed").expect("modify for deleted-type test");
        let v = corpus_git_revert(&json!({ "path": "f.md", "type": "deleted" }));
        assert_eq!(v["ok"], json!(true), "expected ok for deleted type, got: {v}");
        let content = fs::read_to_string(corpus.join("f.md")).expect("read");
        assert_eq!(content, "original", "file not restored for type=deleted");
    });
}
