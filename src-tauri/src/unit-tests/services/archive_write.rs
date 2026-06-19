use std::fs;

use serde_json::json;

use crate::services::archive_write::{archive_digest, archive_document};

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

Summary body here with enough content.
"#;

fn setup_corpus() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let repo_root = dir.path().to_path_buf();
    let corpus = repo_root.join("corpus");
    fs::create_dir_all(corpus.join("raw")).expect("raw dir");
    fs::create_dir_all(corpus.join("digest")).expect("digest dir");
    fs::write(corpus.join("index.json"), br#"{"entries":{}}"#).expect("index");
    crate::config::settings::write_test_config(&repo_root, &corpus, None);
    (dir, repo_root)
}

#[test]
fn archive_document_writes_raw_and_index() {
    let (_dir, repo_root) = setup_corpus();
    let v = archive_document(
        &repo_root,
        &json!({ "document": SAMPLE_DOC, "source_type": "summary" }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "archive_document failed: {v}");
    let id = v["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    let raw = corpus.join("raw/inbox/test-topic/202606191430-test-slug.md");
    assert!(raw.is_file());
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert!(index["entries"][id]["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn archive_document_conflict_returns_409() {
    let (_dir, repo_root) = setup_corpus();
    let payload = json!({ "document": SAMPLE_DOC });
    assert_eq!(archive_document(&repo_root, &payload).get("ok"), Some(&json!(true)));
    let v = archive_document(&repo_root, &payload);
    assert_eq!(v.get("_status"), Some(&json!(409)));
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn archive_digest_writes_digest_and_updates_layers() {
    let (_dir, repo_root) = setup_corpus();
    let created = archive_document(&repo_root, &json!({ "document": SAMPLE_DOC }));
    let id = created["id"].as_str().unwrap();
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview";
    let v = archive_digest(
        &repo_root,
        &json!({ "id": id, "digest": digest_body }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)));
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    let digest = corpus.join("digest/inbox/test-topic/202606191430-test-slug.md");
    assert!(digest.is_file());
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert!(index["entries"][id]["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("digest")));
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn archive_digest_force_overwrites_existing() {
    let (_dir, repo_root) = setup_corpus();
    let created = archive_document(&repo_root, &json!({ "document": SAMPLE_DOC }));
    let id = created["id"].as_str().unwrap();
    let digest_body = "# Test — 摘要\n\n## 概述\n\nv1";
    archive_digest(&repo_root, &json!({ "id": id, "digest": digest_body }));
    let blocked = archive_digest(
        &repo_root,
        &json!({ "id": id, "digest": "# v2\n\n## 概述\n\nblocked" }),
    );
    assert_eq!(blocked.get("_status"), Some(&json!(409)));
    let ok = archive_digest(
        &repo_root,
        &json!({ "id": id, "digest": "# v2\n\n## 概述\n\nforced", "force": true }),
    );
    assert_eq!(ok.get("ok"), Some(&json!(true)));
    crate::config::settings::set_test_config_dir(None);
}
