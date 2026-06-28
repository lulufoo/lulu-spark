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

#[test]
fn kb_list_json_root_listing() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
    fs::write(repo_dir.join("readme.md"), "# hi").expect("w");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_list_json(dir.path(), "lulufoo/myrepo", "", "flat");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    let names: Vec<_> = arr.iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"docs"));
    assert!(names.contains(&"readme.md"));
    let docs = arr.iter().find(|e| e["name"] == "docs").unwrap();
    assert_eq!(docs["relative_path"], "docs");
    assert_eq!(docs["is_dir"], true);
    let readme = arr.iter().find(|e| e["name"] == "readme.md").unwrap();
    assert_eq!(readme["relative_path"], "readme.md");
    assert_eq!(readme["is_dir"], false);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_list_json_empty_directory() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo").join("empty")).expect("mkdir");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_list_json(dir.path(), "lulufoo/myrepo", "empty", "flat");
    assert_eq!(v.as_array().map(|a| a.len()), Some(0));
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_list_json_rejects_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    let v = kb_list_json(dir.path(), "lulufoo/myrepo", "..", "flat");
    assert_eq!(v["error"], "invalid path");
    assert_eq!(v["_status"], 400);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_list_json_repo_not_cloned() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_list_json(dir.path(), "lulufoo/missing", "", "flat");
    assert!(v["error"].as_str().unwrap().contains("not cloned"));
    assert_eq!(v["_status"], 404);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_list_json_invalid_repo_format() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_list_json(dir.path(), "invalidrepo", "", "flat");
    assert_eq!(v["error"], "invalid repo format");
    assert_eq!(v["_status"], 400);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_list_json_rejects_tree_mode() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_list_json(dir.path(), "lulufoo/myrepo", "", "tree");
    assert_eq!(v["error"], "mode tree not implemented");
    assert_eq!(v["_status"], 400);
    crate::config::settings::set_test_config_dir(None);
}
