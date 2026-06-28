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
fn kb_list_docs_returns_only_readable_markdown_entries() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir docs");
    fs::create_dir_all(repo_dir.join(".knowledge_annotations")).expect("mkdir annotations");
    fs::write(repo_dir.join("README.md"), "# Root").expect("write readme");
    fs::write(repo_dir.join("docs/a.md"), "# A").expect("write doc");
    fs::write(repo_dir.join("docs/ignore.txt"), "no").expect("write txt");
    fs::write(repo_dir.join(".knowledge_annotations/a.md.json"), "{}").expect("write ann");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));

    let v = kb_list_docs_json(dir.path(), "lulufoo/myrepo");
    let docs = v["docs"].as_array().expect("docs array");
    let paths: Vec<&str> = docs
        .iter()
        .map(|doc| doc["path"].as_str().expect("path"))
        .collect();

    assert_eq!(paths, vec!["README.md", "docs/a.md"]);
    assert!(docs.iter().all(|doc| doc["repo"] == "lulufoo/myrepo"));
    assert!(docs
        .iter()
        .all(|doc| doc["url"].as_str().unwrap_or("").contains("lulufoo/myrepo")));
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_list_docs_returns_error_for_missing_repo() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(&kb).expect("mkdir kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));

    let v = kb_list_docs_json(dir.path(), "lulufoo/missing");

    assert_eq!(v["_status"], 404);
    assert_eq!(v["error"], "repo not cloned: missing");
    crate::config::settings::set_test_config_dir(None);
}
