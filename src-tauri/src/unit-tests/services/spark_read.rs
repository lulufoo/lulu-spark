use super::*;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::test_support::{seed_note_entries, TestSandbox};

#[test]
fn get_config_has_frontend_contract_keys() {
    let _sandbox = TestSandbox::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let v = get_config(&root);
    assert!(v.get("spark_root").is_some());
    assert!(v.get("workspaces_root").is_some());
    assert!(v.get("knowledge_root").is_none());
    assert!(v.get("notes_root").is_none());
    assert!(v.get("github_user_url").is_none());
    assert!(v.get("spark_github_repo_url").is_none());
    assert!(v.get("assistant_engine").is_some());
    assert!(v.get("theme").is_some());
    assert!(v.get("cache_dir").is_some());
    assert!(v.get("has_github_token").is_none());
    assert!(v.get("has_host_key").is_none());
    assert!(v.get("has_cursor_key").is_none());
    assert!(v.get("llm").is_some());
    assert!(v.get("api_key").is_none());
    assert!(v.get("api_key_host").is_none());
    assert!(v.get("api_key_cursor").is_none());
    assert!(v["llm"].get("api_key").is_none());
    assert!(v["llm"]["host"].get("key_saved").is_some());
}


fn with_notes_repo<F: FnOnce(&std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let notes = sandbox.data_dir().join("workspaces").join("default");
    fs::create_dir_all(&notes).expect("workspace dir");
    setup(cfg_dir, notes.as_path());
    f(cfg_dir);
}

#[test]
fn get_draft_invalid_id() {
    let dir = tempfile::tempdir().expect("tmp");
    let v = get_draft(dir.path(), "..%2Fsecret");
    assert_eq!(v["error"], "Invalid id");
}

#[test]
fn check_spark_root_requires_an_existing_directory() {
    let dir = tempfile::tempdir().expect("tmp");
    assert_eq!(check_spark_root("/no/such/spark")["ok"], false);
    assert_eq!(check_spark_root("  ")["ok"], false);
    assert_eq!(check_spark_root(dir.path().to_str().unwrap())["ok"], true);
}

#[test]
fn get_docs_index_is_built_from_the_doc_table_in_the_entry_shape() {
    with_notes_repo(
        |_, _| {
            seed_note_entries(&json!({
                "n1": {
                    "common_path": "ai/topic/20260101-a.md", "created_at": "202601010000",
                    "layers": ["raw", "digest"], "source_type": "article", "title": "A",
                    "translations": { "zh": "ai/topic/20260101-a-zh.md" }
                },
                "n2": { "common_path": "ai/topic/20260102-b.md", "layers": ["raw"] },
            }));
        },
        |_cfg_dir| {
            let v = get_docs_index(None);
            assert_eq!(
                v,
                json!({ "workspace_id": "default", "entries": {
                    "n1": {
                        "id": "n1", "workspace_id": "default", "project_id": "seed-ai",
                        "raw_path": "topic/20260101-a.md", "created_at": "202601010000",
                        "layers": ["raw", "digest"], "source_type": "article", "title": "A",
                        "translations": { "zh": "topic/20260101-a-zh.md" }
                    },
                    "n2": {
                        "id": "n2", "workspace_id": "default", "project_id": "seed-ai",
                        "raw_path": "topic/20260102-b.md", "layers": ["raw"]
                    },
                }})
            );
        },
    );
}

#[test]
fn get_docs_index_of_an_empty_table_has_no_entries() {
    with_notes_repo(
        |_, _| {},
        |_| assert_eq!(get_docs_index(None), json!({ "workspace_id": "default", "entries": {} })),
    );
}

#[test]
fn get_doc_file_rejects_traversal() {
    with_notes_repo(
        |_, notes| {
            fs::create_dir_all(notes.join("raw")).expect("mkdir");
        },
        |_cfg_dir| {
            let v = get_doc_file("raw", "../index.json", None);
            assert_eq!(v["error"], "Invalid id");
        },
    );
}

#[test]
fn get_annotations_summary_includes_resolved_tags() {
    use crate::services::spark_data::{find_doc, open, update_doc};
    use crate::services::tag_write::tag_attach;
    use crate::test_support::with_sandbox_notes;

    with_sandbox_notes(|dir, _| {
        let cp = "ai/tags.md";
        let id = format!("{:0>32}", "e1");
        seed_note_entries(&serde_json::json!({ id.clone(): { "common_path": cp } }));
        let known = tag_attach(dir, &id, &serde_json::json!({ "value": "Known" }));
        let known = known["key"].as_str().expect("key").to_string();
        // A row can reference a key the tag table does not have.
        let conn = open().expect("db");
        let found = find_doc(&conn, &id).expect("get").expect("row");
        let mut note = found.doc;
        note.tags = serde_json::json!([known, "ghostkey123456"]);
        update_doc(&conn, &found.workspace.doc_table, &note).expect("update");

        let summary = get_annotations_summary(dir);
        let entry = summary[&id].as_object().expect("entry");
        let tags = entry["tags"].as_array().expect("tags");
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().any(|t| t["key"] == known && t["value"] == "Known"));
        assert!(
            tags.iter()
                .any(|t| t["key"] == "ghostkey123456" && t["unknown"] == true)
        );
    });
}

#[test]
fn get_doc_file_returns_content() {
    let id = "cccccccccccccccccccccccccccccccc";
    with_notes_repo(
        move |_, notes| {
            let raw = notes.join("raw").join("inbox").join("a.md");
            fs::create_dir_all(raw.parent().unwrap()).expect("mkdir");
            fs::write(&raw, b"hello").expect("write");
            seed_note_entries(&json!({ id: { "common_path": "a.md" } }));
        },
        move |_cfg_dir| {
            let v = get_doc_file("raw", id, None);
            assert_eq!(v["content"], "hello");
        },
    );
}

#[test]
fn get_doc_asset_returns_png_bytes() {
    let id = "dddddddddddddddddddddddddddddddd";
    with_notes_repo(
        move |_, notes| {
            let png = notes.join("raw").join("seed-ai").join("note.png");
            fs::create_dir_all(png.parent().unwrap()).expect("mkdir");
            fs::write(&png, b"\x89PNG\r\n").expect("write");
            fs::write(notes.join("raw").join("seed-ai").join("note.md"), b"# x").expect("md");
            seed_note_entries(&json!({ id: { "common_path": "ai/note.md" } }));
        },
        move |_cfg_dir| {
            let v = get_doc_asset("raw", id, "note.png", None);
            assert!(v.get("data_b64").and_then(|x| x.as_str()).is_some());
            assert_eq!(v["mime_type"], "image/png");
        },
    );
}

#[test]
fn get_doc_asset_rejects_traversal() {
    let id = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    with_notes_repo(
        move |_, notes| {
            fs::create_dir_all(notes.join("raw").join("seed-ai")).expect("mkdir");
            fs::write(notes.join("raw").join("seed-ai").join("note.md"), b"# x").expect("md");
            seed_note_entries(&json!({ id: { "common_path": "ai/note.md" } }));
        },
        move |_cfg_dir| {
            let v = get_doc_asset("raw", id, "../index.json", None);
            assert_eq!(v["error"], "Invalid path");
        },
    );
}

#[test]
fn get_doc_asset_rejects_non_whitelist_ext() {
    let id = "ffffffffffffffffffffffffffffffff";
    with_notes_repo(
        move |_, notes| {
            let svg = notes.join("raw").join("inbox").join("x.svg");
            fs::create_dir_all(svg.parent().unwrap()).expect("mkdir");
            fs::write(&svg, b"<svg").expect("write");
            fs::write(notes.join("raw").join("inbox").join("x.md"), b"#").expect("md");
            seed_note_entries(&json!({ id: { "common_path": "x.md" } }));
        },
        move |_cfg_dir| {
            let v = get_doc_asset("raw", id, "x.svg", None);
            assert_eq!(v["error"], "Unsupported media type");
        },
    );
}

#[test]
fn get_annotation_decodes_percent_encoding() {
    with_notes_repo(
        |_, _| {
            let id = "a".repeat(32);
            seed_note_entries(&json!({ id.clone(): { "common_path": "ai/note.md" } }));
            let dir = std::path::Path::new("");
            assert_eq!(crate::services::annotation::set_done(dir, &id, true)["ok"], json!(true));
        },
        |_cfg_dir| {
            let id = "a".repeat(32);
            let v = get_annotation(_cfg_dir, &id);
            assert_eq!(v["done"], json!(true));
        },
    );
}
