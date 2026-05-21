use super::*;
use std::fs;

#[test]
fn rejects_path_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = kb_safe_path(&kb, "lulufoo/myrepo", "../secret.md").unwrap_err();
    assert_eq!(err, "invalid path");
}

#[test]
fn resolves_file_under_repo() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(&repo_dir).expect("mkdir");
    let md = repo_dir.join("docs/a.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# hi").expect("write");
    let p = kb_safe_path(&kb, "lulufoo/myrepo", "docs/a.md").expect("ok");
    assert!(p.is_file());
}
