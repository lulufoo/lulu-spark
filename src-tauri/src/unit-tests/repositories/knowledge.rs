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

#[test]
fn kb_safe_path_rejects_empty_path() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = kb_safe_path(&kb, "lulufoo/myrepo", "").unwrap_err();
    assert_eq!(err, "invalid path");
}

#[test]
fn kb_list_dir_allows_empty_path_for_repo_root() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(&repo_dir).expect("mkdir");
    let p = kb_list_dir(&kb, "lulufoo/myrepo", "").expect("ok");
    assert!(p.is_dir());
    assert_eq!(p, repo_dir.canonicalize().unwrap_or(repo_dir));
}

#[test]
fn resolves_file_under_directory_name() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("topic-name");
    fs::create_dir_all(&repo_dir).expect("mkdir");
    let md = repo_dir.join("docs/a.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# hi").expect("write");
    let p = kb_safe_path(&kb, "topic-name", "docs/a.md").expect("ok");
    assert!(p.is_file());
}

#[test]
fn kb_list_dir_rejects_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = kb_list_dir(&kb, "lulufoo/myrepo", "..").unwrap_err();
    assert_eq!(err, "invalid path");
}
