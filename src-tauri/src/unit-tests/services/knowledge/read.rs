use super::*;
use crate::test_support::TestSandbox;
use std::fs;

fn with_kb_repo<F: FnOnce(&std::path::Path, &std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let kb = sandbox.knowledge_root();
    setup(cfg_dir, &kb);
    f(cfg_dir, &kb);
}

fn with_kb_and_sediment_cache<F: FnOnce(&std::path::Path, &std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let kb = sandbox.knowledge_root();
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
            assert!(v.get("committed_at").is_none());
            let by_dir = kb_read_json(cfg_dir, "myrepo", "docs/a.md");
            assert_eq!(by_dir["content"], "hello");
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
fn kb_list_json_directory_not_found() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "lulufoo/missing", "", "flat");
            assert!(v["error"].as_str().unwrap().contains("directory not found"));
            assert_eq!(v["_status"], 404);
        },
    );
}

#[test]
fn kb_list_json_invalid_directory_name() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_list_json(cfg_dir, "../secret", "", "flat");
            assert_eq!(v["error"], "invalid directory name");
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
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None);
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
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None);
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
            crate::services::knowledge::add_kb_hide_pattern("^draft").expect("add");
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None);
            assert_eq!(v["count"], 2);
        },
    );
}

#[test]
fn kb_doc_count_json_skips_hidden_directory() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join(".cache")).expect("mkdir");
            fs::write(repo_dir.join("keep.md"), "k").expect("w");
            fs::write(repo_dir.join(".cache/hidden.md"), "h").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None);
            assert_eq!(v["count"], 1);
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
            let v = kb_doc_count_json(cfg_dir, "lulufoo/myrepo", None);
            assert_eq!(v["count"], 2);
        },
    );
}

#[test]
fn kb_doc_count_json_category_aggregates() {
    use crate::services::sediment_kb::{
        add_category, add_directory, ensure_uncategorized, update_repo_category,
    };

    with_kb_and_sediment_cache(|dir, kb| {
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Research").expect("cat");
        add_directory("repoa").expect("add a");
        add_directory("repob").expect("add b");
        update_repo_category("repoa", &cat_id).expect("cat a");
        update_repo_category("repob", &cat_id).expect("cat b");

        let repo_a = kb.join("repoa");
        let repo_b = kb.join("repob");
        fs::create_dir_all(repo_a.join("docs")).expect("mkdir a");
        fs::create_dir_all(repo_b.join("docs")).expect("mkdir b");
        fs::write(repo_a.join("one.md"), "1").expect("w");
        fs::write(repo_a.join("docs/two.md"), "2").expect("w");
        fs::write(repo_b.join("docs/three.md"), "3").expect("w");

        let v = kb_doc_count_json(dir, "repoa", Some(&cat_id));
        assert_eq!(v["count"], 3);
        assert!(v.get("error").is_none());
    });
}

#[test]
fn kb_doc_count_json_directory_not_found() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/missing", None);
            assert!(v["error"].as_str().unwrap().contains("directory not found"));
            assert_eq!(v["_status"], 404);
        },
    );
}

#[test]
fn kb_doc_count_json_rejects_traversal_in_repo() {
    with_kb_repo(
        |_, _kb| {},
        |cfg_dir, _| {
            let v = kb_doc_count_json(cfg_dir, "lulufoo/../secret", None);
            assert_eq!(v["error"], "invalid directory name");
            assert_eq!(v["_status"], 400);
        },
    );
}
