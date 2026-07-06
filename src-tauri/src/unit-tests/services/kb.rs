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

fn with_kb_and_sediment_cache<F: FnOnce(&std::path::Path, &std::path::Path)>(f: F) {
    crate::test_support::with_test_config_dir(|cfg| {
        let cache = cfg.join("cache");
        let kb = cfg.join("kb");
        fs::write(
            cfg.join("config.toml"),
            format!(
                r#"workbench_knowledge_root = "{}"
knowledge_corpus_root = "{}"
cache_dir = "{}"
"#,
                cfg.display(),
                kb.display(),
                cache.display(),
            ),
        )
        .expect("write config");
        f(cfg, &kb);
    });
}

#[test]
fn kb_doc_count_json_counts_md_recursively() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
    fs::write(repo_dir.join("a.md"), "a").expect("w");
    fs::write(repo_dir.join("docs/b.md"), "b").expect("w");
    fs::write(repo_dir.join("docs/c.md"), "c").expect("w");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/myrepo", None, None);
    assert_eq!(v["count"], 3);
    assert!(v.get("error").is_none());
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_empty_repo() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/myrepo", None, None);
    assert_eq!(v["count"], 0);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_hide_pattern_excludes_matching() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
    fs::write(repo_dir.join("keep.md"), "k").expect("w");
    fs::write(repo_dir.join("draft.md"), "d").expect("w");
    fs::write(repo_dir.join("docs/also.md"), "a").expect("w");
    fs::write(repo_dir.join("docs/draft-notes.md"), "n").expect("w");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/myrepo", Some("^draft"), None);
    assert_eq!(v["count"], 2);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_invalid_hide_pattern_ignored() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
    fs::write(repo_dir.join("a.md"), "a").expect("w");
    fs::write(repo_dir.join("b.md"), "b").expect("w");
    fs::write(repo_dir.join("draft.md"), "d").expect("w");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/myrepo", Some("["), None);
    assert_eq!(v["count"], 3);
    assert!(v.get("error").is_none());
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_only_counts_md() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo_dir = kb.join("myrepo");
    fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
    fs::write(repo_dir.join("readme.md"), "# hi").expect("w");
    fs::write(repo_dir.join("notes.txt"), "txt").expect("w");
    fs::write(repo_dir.join("docs/guide.md"), "g").expect("w");
    fs::write(repo_dir.join("docs/image.png"), "png").expect("w");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/myrepo", None, None);
    assert_eq!(v["count"], 2);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_category_aggregates() {
    use crate::services::sediment_kb::{
        add_category, add_repo, ensure_uncategorized, set_test_repo_validator, SedimentKbError,
    };

    fn ok_validator(full_name: &str) -> Result<String, SedimentKbError> {
        Ok(full_name.to_string())
    }

    with_kb_and_sediment_cache(|dir, kb| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Research").expect("cat");
        add_repo("owner/repoa", Some(&cat_id), "").expect("add a");
        add_repo("owner/repob", Some(&cat_id), "").expect("add b");

        let repo_a = kb.join("repoa");
        let repo_b = kb.join("repob");
        fs::create_dir_all(repo_a.join("docs")).expect("mkdir a");
        fs::create_dir_all(repo_b.join("docs")).expect("mkdir b");
        fs::write(repo_a.join("one.md"), "1").expect("w");
        fs::write(repo_a.join("docs/two.md"), "2").expect("w");
        fs::write(repo_b.join("docs/three.md"), "3").expect("w");

        let v = kb_doc_count_json(dir, "owner/repoa", None, Some(&cat_id));
        assert_eq!(v["count"], 3);
        assert!(v.get("error").is_none());
        set_test_repo_validator(None);
    });
}

#[test]
fn kb_doc_count_json_repo_not_cloned() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/missing", None, None);
    assert!(v["error"].as_str().unwrap().contains("not cloned"));
    assert_eq!(v["_status"], 404);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_invalid_repo_format() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "invalidrepo", None, None);
    assert_eq!(v["error"], "invalid repo format");
    assert_eq!(v["_status"], 400);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn kb_doc_count_json_rejects_traversal_in_repo() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    crate::config::settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_doc_count_json(dir.path(), "lulufoo/../secret", None, None);
    assert_eq!(v["error"], "invalid repo format");
    assert_eq!(v["_status"], 400);
    crate::config::settings::set_test_config_dir(None);
}
