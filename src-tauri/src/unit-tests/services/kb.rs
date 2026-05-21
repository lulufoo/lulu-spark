use super::*;
use std::fs;

#[test]
fn kb_read_returns_content() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
    fs::write(repo_dir.join("docs/a.md"), "hello").expect("w");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_read_json(dir.path(), "lulufoo/myrepo", "docs/a.md");
    assert_eq!(v["content"], "hello");
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_read_rejects_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let v = kb_read_json(dir.path(), "lulufoo/myrepo", "../x.md");
    assert_eq!(v["error"], "invalid path");
    crate::config::settings::set_test_config_dir(None);
}
