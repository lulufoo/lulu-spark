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

#[test]
fn kb_asset_json_returns_png_bytes() {
    with_kb_repo(
        |_, kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
            fs::write(dir.join("note.png"), b"\x89PNG\r\n").expect("png");
        },
        |cfg_dir, _| {
            let v = kb_asset_json(cfg_dir, "lulufoo/myrepo", "docs/a.md", "note.png");
            assert!(v.get("data_b64").and_then(|x| x.as_str()).is_some());
            assert_eq!(v["mime_type"], "image/png");
        },
    );
}

#[test]
fn kb_asset_json_accepts_dot_slash_href() {
    with_kb_repo(
        |_, kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
            fs::write(dir.join("note.png"), b"\x89PNG\r\n").expect("png");
        },
        |cfg_dir, _| {
            let v = kb_asset_json(cfg_dir, "lulufoo/myrepo", "docs/a.md", "./note.png");
            assert_eq!(v["mime_type"], "image/png");
            assert!(v.get("error").is_none());
        },
    );
}

#[test]
fn kb_asset_json_rejects_traversal() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo").join("docs")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_asset_json(cfg_dir, "lulufoo/myrepo", "docs/a.md", "../secret.png");
            assert_eq!(v["error"], "invalid path");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_asset_json_rejects_non_whitelist_ext() {
    with_kb_repo(
        |_, kb| {
            let dir = kb.join("myrepo");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("x.md"), "#").expect("md");
            fs::write(dir.join("x.svg"), b"<svg").expect("svg");
        },
        |cfg_dir, _| {
            let v = kb_asset_json(cfg_dir, "lulufoo/myrepo", "x.md", "x.svg");
            assert_eq!(v["error"], "Unsupported media type");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_asset_json_missing_file_is_404() {
    with_kb_repo(
        |_, kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
        },
        |cfg_dir, _| {
            let v = kb_asset_json(cfg_dir, "lulufoo/myrepo", "docs/a.md", "missing.png");
            assert!(v["error"].as_str().unwrap().contains("file not found"));
            assert_eq!(v["_status"], 404);
        },
    );
}
