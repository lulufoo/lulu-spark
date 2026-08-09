use super::*;
use crate::config::settings;
use crate::test_support::TestSandbox;
use std::fs;

fn with_kb_repo<F: FnOnce(&std::path::Path, &std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let config_path = sandbox.config_file_path();
    let (http_port, mcp_port) = sandbox.ports();
    let kb = cfg_dir.join("kb");
    setup(cfg_dir, &kb);
    settings::write_test_config_with_cache(
        &config_path,
        cfg_dir,
        Some(&kb),
        Some(&cfg_dir.join("cache")),
        "http://127.0.0.1:17700",
        http_port,
        mcp_port,
    )
    .expect("write kb config");
    f(cfg_dir, &kb);
}

fn with_kb_and_sediment_cache<F: FnOnce(&std::path::Path, &std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let config_path = sandbox.config_file_path();
    let (http_port, mcp_port) = sandbox.ports();
    let cache = cfg_dir.join("cache");
    let kb = cfg_dir.join("kb");
    settings::write_test_config_with_cache(
        &config_path,
        cfg_dir,
        Some(&kb),
        Some(&cache),
        "http://127.0.0.1:17700",
        http_port,
        mcp_port,
    )
    .expect("write kb config");
    f(cfg_dir, &kb);
}

#[test]
fn kb_read_returns_content() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("docs/a.md"), "hello").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_read_json(cfg_dir, "lulufoo/myrepo", "docs/a.md");
            assert_eq!(v["content"], "hello");
        },
    );
}

#[test]
fn kb_read_rejects_traversal() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_read_json(cfg_dir, "lulufoo/myrepo", "../x.md");
            assert_eq!(v["error"], "invalid path");
        },
    );
}

#[test]
fn kb_list_json_root_listing() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("readme.md"), "# hi").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "lulufoo/myrepo", "", "flat");
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
        },
    );
}

#[test]
fn kb_list_json_empty_directory() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo").join("empty")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "lulufoo/myrepo", "empty", "flat");
            assert_eq!(v.as_array().map(|a| a.len()), Some(0));
        },
    );
}

#[test]
fn kb_list_json_rejects_traversal() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "lulufoo/myrepo", "..", "flat");
            assert_eq!(v["error"], "invalid path");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_list_json_repo_not_cloned() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "lulufoo/missing", "", "flat");
            assert!(v["error"].as_str().unwrap().contains("not cloned"));
            assert_eq!(v["_status"], 404);
        },
    );
}

#[test]
fn kb_list_json_invalid_repo_format() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "invalidrepo", "", "flat");
            assert_eq!(v["error"], "invalid repo format");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_list_json_rejects_tree_mode() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "lulufoo/myrepo", "", "tree");
            assert_eq!(v["error"], "mode tree not implemented");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_doc_count_json_counts_md_recursively() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("a.md"), "a").expect("w");
            fs::write(repo_dir.join("docs/b.md"), "b").expect("w");
            fs::write(repo_dir.join("docs/c.md"), "c").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None, None);
            assert_eq!(v["count"], 3);
            assert!(v.get("error").is_none());
        },
    );
}

#[test]
fn kb_doc_count_json_empty_repo() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None, None);
            assert_eq!(v["count"], 0);
        },
    );
}

#[test]
fn kb_doc_count_json_hide_pattern_excludes_matching() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("keep.md"), "k").expect("w");
            fs::write(repo_dir.join("draft.md"), "d").expect("w");
            fs::write(repo_dir.join("docs/also.md"), "a").expect("w");
            fs::write(repo_dir.join("docs/draft-notes.md"), "n").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", Some("^draft"), None);
            assert_eq!(v["count"], 2);
        },
    );
}

#[test]
fn kb_doc_count_json_invalid_hide_pattern_ignored() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("a.md"), "a").expect("w");
            fs::write(repo_dir.join("b.md"), "b").expect("w");
            fs::write(repo_dir.join("draft.md"), "d").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", Some("["), None);
            assert_eq!(v["count"], 3);
            assert!(v.get("error").is_none());
        },
    );
}

#[test]
fn kb_doc_count_json_only_counts_md() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("readme.md"), "# hi").expect("w");
            fs::write(repo_dir.join("notes.txt"), "txt").expect("w");
            fs::write(repo_dir.join("docs/guide.md"), "g").expect("w");
            fs::write(repo_dir.join("docs/image.png"), "png").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None, None);
            assert_eq!(v["count"], 2);
        },
    );
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
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/missing", None, None);
            assert!(v["error"].as_str().unwrap().contains("not cloned"));
            assert_eq!(v["_status"], 404);
        },
    );
}

#[test]
fn kb_doc_count_json_invalid_repo_format() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "invalidrepo", None, None);
            assert_eq!(v["error"], "invalid repo format");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_doc_count_json_rejects_traversal_in_repo() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/../secret", None, None);
            assert_eq!(v["error"], "invalid repo format");
            assert_eq!(v["_status"], 400);
        },
    );
}
