use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::docs::{create_doc, create_doc_content, delete_doc};
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30

---

摘要正文，enough content.
"#;

const PNG: &[u8] = b"\x89PNG\r\n";

fn setup_notes() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    sandbox
}

fn stage_bytes(sandbox: &TestSandbox, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, bytes).expect("write");
    p.canonicalize().expect("canon")
}

fn theme_dir() -> PathBuf {
    crate::test_support::default_workspace_dir()
        .join("raw")
        .join(crate::test_support::default_inbox_id())
        .join("notes")
}

#[test]
fn delete_doc_removes_the_attached_dir() {
    let sandbox = setup_notes();
    let source = stage_bytes(&sandbox, "source.md", SAMPLE_DOC.as_bytes());
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let created = create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [png.to_str().unwrap()],
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let attached = theme_dir().join(".attached").join(id);
    assert!(attached.join("overview-diagram.png").is_file());
    let deleted = delete_doc(&json!({ "id": id }));
    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert!(!attached.exists());
    assert!(!theme_dir().join(".attached").exists());
}

#[test]
fn delete_doc_without_an_attached_dir_still_succeeds() {
    let _sandbox = setup_notes();
    let created = create_doc_content(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "title": "T",
            "content": "# 标题\n\n这是正文，足够写入。\n",
            "digest": "never",
            "source_type": "summary",
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    assert!(!theme_dir().join(".attached").join(id).exists());
    let deleted = delete_doc(&json!({ "id": id }));
    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
}
