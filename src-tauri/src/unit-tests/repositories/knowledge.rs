use super::*;
use std::fs;

#[test]
fn rejects_path_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = knowledge_file_path(&kb, "myrepo", "../secret.md").unwrap_err();
    assert_eq!(err, "invalid path");
}

#[test]
fn resolves_file_under_folder() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let topic = kb.join("myrepo");
    fs::create_dir_all(&topic).expect("mkdir");
    let md = topic.join("docs/a.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# hi").expect("write");
    let p = knowledge_file_path(&kb, "myrepo", "docs/a.md").expect("ok");
    assert!(p.is_file());
}

#[test]
fn knowledge_file_path_rejects_empty_path() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = knowledge_file_path(&kb, "myrepo", "").unwrap_err();
    assert_eq!(err, "invalid path");
}

#[test]
fn knowledge_file_path_rejects_slash_in_folder() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = knowledge_file_path(&kb, "lulufoo/myrepo", "docs/a.md").unwrap_err();
    assert_eq!(err, "invalid directory name");
}

#[test]
fn knowledge_dir_path_allows_empty_rel_for_topic_root() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let topic = kb.join("myrepo");
    fs::create_dir_all(&topic).expect("mkdir");
    let p = knowledge_dir_path(&kb, "myrepo", "").expect("ok");
    assert!(p.is_dir());
    assert_eq!(p, topic.canonicalize().unwrap_or(topic));
}

#[test]
fn knowledge_dir_path_rejects_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let err = knowledge_dir_path(&kb, "myrepo", "..").unwrap_err();
    assert_eq!(err, "invalid path");
}
