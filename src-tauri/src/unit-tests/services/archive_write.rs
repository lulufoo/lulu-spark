use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::services::archive_write::{archive_digest, archive_document};
use crate::services::plan_task::{
    create_master_with_subs, get_by_id, test_set_fail_complete_sub, test_set_fail_link_archive,
};
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

Summary body here with enough content.
"#;

fn setup_corpus() -> (TestSandbox, std::path::PathBuf) {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(wb.join("raw")).expect("raw dir");
    fs::create_dir_all(wb.join("digest")).expect("digest dir");
    fs::write(wb.join("index.json"), br#"{"entries":{}}"#).expect("index");
    let repo_root = sandbox.config_dir().to_path_buf();
    (sandbox, repo_root)
}

fn setup_corpus_with_plan_tasks() -> (TestSandbox, PathBuf) {
    setup_corpus()
}

fn read_v2_index(wb: &Path) -> serde_json::Value {
    serde_json::from_str(
        &fs::read_to_string(wb.join("plan_tasks").join("index.json")).expect("index.json"),
    )
    .expect("parse index.json")
}

fn with_archive_plan_task_test<F: FnOnce()>(f: F) {
    f();
}

#[test]
fn archive_document_writes_raw_and_index() {
    let (_sandbox, repo_root) = setup_corpus();
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
}

#[test]
fn archive_document_conflict_returns_409() {
    let (_sandbox, repo_root) = setup_corpus();
    let payload = json!({ "document": SAMPLE_DOC });
    assert_eq!(archive_document(&repo_root, &payload).get("ok"), Some(&json!(true)));
    let v = archive_document(&repo_root, &payload);
    assert_eq!(v.get("_status"), Some(&json!(409)));
}

#[test]
fn archive_digest_writes_digest_and_updates_layers() {
    let (_sandbox, repo_root) = setup_corpus();
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
}

#[test]
fn archive_digest_force_overwrites_existing() {
    let (_sandbox, repo_root) = setup_corpus();
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
}

const THEME_LINE_DOC: &str = r#"# Interview Title

> 创建时间：2026年6月19日 17:00
> 时长：约 30 分钟 · 发布：2026-01-01
> 导航：[digest](../../../digest/learning-ai-agent/waymo-interview/202606191700-waymo-interview.md)
> 原文：[Video](https://example.com/watch)

---

## Theme one
Host: hello
"#;

const THEME_LINE_ZH: &str = r#"# 中文标题

> 创建时间：2026年6月19日 17:00

---

## 主题一
主持人：你好
"#;

#[test]
fn archive_document_theme_line_with_zh_extra() {
    let (_sandbox, repo_root) = setup_corpus();
    let zh_path = "learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md";
    let v = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "source_type": "theme-line",
            "extra_documents": [{
                "rel": format!("raw/{zh_path}"),
                "content": THEME_LINE_ZH
            }],
            "index_extra": { "translations": { "zh": zh_path } }
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    assert!(corpus
        .join("raw/learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md")
        .is_file());
    let id = v["id"].as_str().unwrap();
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert_eq!(
        index["entries"][id]["translations"]["zh"],
        json!(zh_path)
    );
}

#[test]
fn archive_document_rejects_mismatched_zh_path() {
    let (_sandbox, repo_root) = setup_corpus();
    let v = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "source_type": "theme-line",
            "extra_documents": [{
                "rel": "raw/learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md",
                "content": THEME_LINE_ZH
            }],
            "index_extra": { "translations": { "zh": "wrong/path.md" } }
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn archive_document_with_task_ref_completes_sub_in_sandbox() {
    with_archive_plan_task_test(|| {
        let (_sandbox, repo_root) = setup_corpus();
        let wb = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
        let created = create_master_with_subs("Archive link", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["sub_task_id"].as_str().unwrap();

        let v = archive_document(
            &repo_root,
            &json!({
                "document": SAMPLE_DOC,
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        );
        assert_eq!(v.get("ok"), Some(&json!(true)), "archive failed: {v}");
        let archive_id = v["id"].as_str().expect("id");

        let index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(wb.join("index.json")).unwrap()).unwrap();
        let entry = &index["entries"][archive_id];
        assert_eq!(entry["task_ref"]["master_task_id"], master_id);
        assert_eq!(entry["task_ref"]["sub_task_id"], sub_id);

        let task = get_by_id(master_id);
        let sub = &task["sub_tasks"][0];
        assert_eq!(sub["status"], "complete");
        assert_eq!(
            sub["linked_archive_ids"].as_array().unwrap()[0],
            archive_id
        );

        let index = read_v2_index(&wb);
        assert_eq!(index["version"], 2);
        assert!(index["tasks"].get(master_id).is_some());
        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(task_dir.join("sub_tasks.json").is_file());
    });
}

#[test]
fn archive_document_plan_task_fail_dual_store_rollback() {
    with_archive_plan_task_test(|| {
        let (_sandbox, repo_root) = setup_corpus();
        let wb = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
        let created = create_master_with_subs("A2 rollback", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["sub_task_id"].as_str().unwrap();

        let index_before: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(wb.join("index.json")).unwrap()).unwrap();

        test_set_fail_complete_sub(true);
        let result = archive_document(
            &repo_root,
            &json!({
                "document": SAMPLE_DOC,
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        );

        assert_ne!(result.get("ok"), Some(&json!(true)), "expected plan_task failure: {result}");
        assert_eq!(result.get("_status"), Some(&json!(500)));

        let raw = wb.join("raw/inbox/test-topic/202606191430-test-slug.md");
        assert!(!raw.exists(), "orphan markdown must be removed after rollback");

        let index_after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(wb.join("index.json")).unwrap()).unwrap();
        assert_eq!(
            index_after, index_before,
            "index must restore to pre-request snapshot"
        );

        let task = get_by_id(master_id);
        assert_eq!(task["sub_tasks"][0]["status"], "incomplete");
        assert_eq!(
            task["sub_tasks"][0]["linked_archive_ids"]
                .as_array()
                .unwrap()
                .len(),
            0
        );

    });
}

#[test]
fn archive_document_index_snapshot_restore_on_plan_task_fail() {
    with_archive_plan_task_test(|| {
        let (_sandbox, repo_root) = setup_corpus();
        let wb = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
        let created = create_master_with_subs("Index snapshot", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["sub_task_id"].as_str().unwrap();

        let index_path = wb.join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["entries"]["seed_entry_00000000000000000000000001"] = json!({
            "common_path": "inbox/seed/existing.md",
            "created_at": "2026年1月1日 00:00",
            "layers": ["raw"],
            "source_type": "summary"
        });
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
        let index_before = index.clone();

        test_set_fail_complete_sub(true);
        let result = archive_document(
            &repo_root,
            &json!({
                "document": SAMPLE_DOC,
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        );
        assert_ne!(result.get("ok"), Some(&json!(true)));

        let index_after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(
            index_after, index_before,
            "rollback must restore full entries map including pre-existing keys"
        );

    });
}

#[test]
fn archive_document_link_fail_corpus_rollback_plan_stays_complete() {
    with_archive_plan_task_test(|| {
        let (_sandbox, repo_root) = setup_corpus();
        let wb = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
        let created = create_master_with_subs("Link fail", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["sub_task_id"].as_str().unwrap();

        let index_before: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(wb.join("index.json")).unwrap()).unwrap();

        test_set_fail_link_archive(true);
        let result = archive_document(
            &repo_root,
            &json!({
                "document": SAMPLE_DOC,
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        );

        assert_ne!(result.get("ok"), Some(&json!(true)), "expected link failure: {result}");
        assert_eq!(result.get("_status"), Some(&json!(500)));

        let raw = wb.join("raw/inbox/test-topic/202606191430-test-slug.md");
        assert!(!raw.exists(), "corpus raw must rollback on link failure");

        let index_after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(wb.join("index.json")).unwrap()).unwrap();
        assert_eq!(index_after, index_before, "corpus index must restore to pre-request snapshot");

        let task = get_by_id(master_id);
        let sub = &task["sub_tasks"][0];
        assert_eq!(sub["status"], "complete", "complete_sub succeeded before link failure");
        assert_eq!(
            sub["linked_archive_ids"].as_array().unwrap().len(),
            0,
            "link_archive must not append on failure"
        );
    });
}
