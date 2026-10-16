use super::*;
use crate::services::spark_data::{self, register_knowledge_doc};
use crate::test_support::TestSandbox;
use std::fs;

fn with_kb_doc<F: FnOnce(&str)>(
    setup: impl FnOnce(&std::path::Path),
    folder: &str,
    rel: &str,
    f: F,
) {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    setup(&kb);
    let id = register_knowledge_doc(&spark_data::open().expect("db"), folder, rel).expect("register");
    f(&id);
}

#[test]
fn kb_asset_json_returns_png_bytes() {
    with_kb_doc(
        |kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
            fs::write(dir.join("note.png"), b"\x89PNG\r\n").expect("png");
        },
        "myrepo",
        "docs/a.md",
        |id| {
            let v = kb_asset_json(id, "note.png");
            assert!(v.get("data_b64").and_then(|x| x.as_str()).is_some());
            assert_eq!(v["mime_type"], "image/png");
        },
    );
}

#[test]
fn kb_asset_json_accepts_dot_slash_href() {
    with_kb_doc(
        |kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
            fs::write(dir.join("note.png"), b"\x89PNG\r\n").expect("png");
        },
        "myrepo",
        "docs/a.md",
        |id| {
            let v = kb_asset_json(id, "./note.png");
            assert_eq!(v["mime_type"], "image/png");
            assert!(v.get("error").is_none());
        },
    );
}

#[test]
fn kb_asset_json_rejects_traversal() {
    with_kb_doc(
        |kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
        },
        "myrepo",
        "docs/a.md",
        |id| {
            let v = kb_asset_json(id, "../secret.png");
            assert_eq!(v["error"], "invalid path");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_asset_json_rejects_non_whitelist_ext() {
    with_kb_doc(
        |kb| {
            let dir = kb.join("myrepo");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("x.md"), "#").expect("md");
            fs::write(dir.join("x.svg"), b"<svg").expect("svg");
        },
        "myrepo",
        "x.md",
        |id| {
            let v = kb_asset_json(id, "x.svg");
            assert_eq!(v["error"], "Unsupported media type");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_asset_json_missing_file_is_404() {
    with_kb_doc(
        |kb| {
            let dir = kb.join("myrepo").join("docs");
            fs::create_dir_all(&dir).expect("mkdir");
            fs::write(dir.join("a.md"), "hello").expect("md");
        },
        "myrepo",
        "docs/a.md",
        |id| {
            let v = kb_asset_json(id, "missing.png");
            assert!(v["error"].as_str().unwrap().contains("file not found"));
            assert_eq!(v["_status"], 404);
        },
    );
}
