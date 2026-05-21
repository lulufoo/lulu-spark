use super::*;
use std::fs;
use std::path::Path;

fn init_repo(dir: &Path) {
    exec(dir, &["init"]).expect("init");
    exec(dir, &["config", "user.email", "t@t.com"]).expect("email");
    exec(dir, &["config", "user.name", "t"]).expect("name");
    fs::write(dir.join("a.txt"), "one").expect("w");
    exec(dir, &["add", "a.txt"]).expect("add");
    exec(dir, &["commit", "-m", "init"]).expect("commit");
}

#[test]
fn status_porcelain_detects_untracked() {
    let dir = tempfile::tempdir().expect("tmp");
    init_repo(dir.path());
    fs::write(dir.path().join("b.txt"), "x").expect("w");
    let out = status_porcelain(dir.path()).expect("status");
    assert!(out.contains("b.txt"));
}

#[test]
fn commit_nothing_to_commit_on_clean_tree() {
    let dir = tempfile::tempdir().expect("tmp");
    init_repo(dir.path());
    let (nothing, _) = commit(dir.path(), "empty").expect("commit");
    assert!(nothing);
}
