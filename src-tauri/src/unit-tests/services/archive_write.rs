use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::services::archive_parse::parse_archive_document;
use crate::services::archive_write::{
    archive_digest, archive_document, archive_note_document, synthesize_note_archive_document,
    NoteCreateOpts,
};
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
        let created = create_master_with_subs("Archive link", Some(&["Sub"]));
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
        let created = create_master_with_subs("A2 rollback", Some(&["Sub"]));
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
        let created = create_master_with_subs("Index snapshot", Some(&["Sub"]));
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
        let created = create_master_with_subs("Link fail", Some(&["Sub"]));
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

fn note_opts_with_ts(ts: &str) -> NoteCreateOpts {
    NoteCreateOpts {
        ts: Some(ts.to_string()),
        ..NoteCreateOpts::default()
    }
}

#[test]
fn synthesize_note_archive_document_builds_parseable_shell() {
    let body = "First line title\n\nMore body content.";
    let doc = synthesize_note_archive_document(body, &note_opts_with_ts("202607101430"))
        .expect("synthesize");
    let parsed = parse_archive_document(&doc).expect("parse must accept synthesized shell");
    assert_eq!(
        parsed.common_path,
        "inbox/notes/202607101430-first-line-title.md"
    );
    assert!(doc.starts_with("# First line title\n"), "H1 from first line: {doc}");
    assert!(
        doc.contains("> 创建时间：2026年7月10日 14:30"),
        "created_at line: {doc}"
    );
    assert!(
        doc.contains("[digest](../../../digest/inbox/notes/202607101430-first-line-title.md)"),
        "digest nav: {doc}"
    );
    assert!(
        doc.contains("---\n\nFirst line title\n\nMore body content."),
        "body preserved after separator: {doc}"
    );
}

#[test]
fn synthesize_note_archive_document_empty_first_line_falls_back_to_ts_slug() {
    let body = "\n\nBody without a title line.";
    let doc = synthesize_note_archive_document(body, &note_opts_with_ts("202607101431"))
        .expect("synthesize");
    let parsed = parse_archive_document(&doc).expect("parse");
    assert_eq!(
        parsed.common_path,
        "inbox/notes/202607101431-note.md"
    );
    assert!(
        doc.starts_with("# 202607101431-note\n") || doc.starts_with("# note\n"),
        "H1 falls back to timestamp/slug: {doc}"
    );
}

#[test]
fn synthesize_note_archive_document_defaults_topic_inbox() {
    let doc = synthesize_note_archive_document("Hello note", &note_opts_with_ts("202607101432"))
        .expect("synthesize");
    let parsed = parse_archive_document(&doc).expect("parse");
    assert!(
        parsed.common_path.starts_with("inbox/"),
        "topic must default to inbox: {}",
        parsed.common_path
    );
}

#[test]
fn archive_note_document_writes_raw_index_with_source_type_note() {
    let (_sandbox, repo_root) = setup_corpus();
    let body = "Quick capture\n\nDetails here.";
    let v = archive_note_document(&repo_root, body, &note_opts_with_ts("202607101433"))
        .expect("archive_note_document");
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let id = v["id"].as_str().expect("id");
    let common_path = v["common_path"].as_str().expect("common_path");
    assert_eq!(common_path, "inbox/notes/202607101433-quick-capture.md");

    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    let raw = corpus.join("raw").join(common_path);
    assert!(raw.is_file(), "raw must exist at {raw:?}");
    let raw_text = fs::read_to_string(&raw).expect("read raw");
    assert!(raw_text.contains("Quick capture"));
    parse_archive_document(&raw_text).expect("stored doc must remain parseable");

    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    let entry = &index["entries"][id];
    assert_eq!(entry["source_type"], json!("note"));
    assert_eq!(entry["common_path"], json!(common_path));
    assert!(entry["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
    assert!(
        !corpus.join("annotations").exists()
            || fs::read_dir(corpus.join("annotations"))
                .map(|mut d| d.next().is_none())
                .unwrap_or(true),
        "must not write Annotation path"
    );
}

#[test]
fn archive_note_document_conflict_does_not_mutate_history() {
    let (_sandbox, repo_root) = setup_corpus();
    let opts = note_opts_with_ts("202607101434");
    let body = "Same path note";
    let first = archive_note_document(&repo_root, body, &opts).expect("first create");
    assert_eq!(first.get("ok"), Some(&json!(true)));
    let id = first["id"].as_str().unwrap().to_string();
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    let index_before: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    let raw_before = fs::read_to_string(
        corpus.join("raw/inbox/notes/202607101434-same-path-note.md"),
    )
    .unwrap();

    let err = archive_note_document(&repo_root, body, &opts).expect_err("409 conflict");
    assert!(
        err.contains("409") || err.to_lowercase().contains("already exists"),
        "expected conflict error: {err}"
    );

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert_eq!(index_after, index_before, "history Entry must not change on conflict");
    assert_eq!(
        index_after["entries"][&id]["source_type"],
        json!("note")
    );
    let raw_after = fs::read_to_string(
        corpus.join("raw/inbox/notes/202607101434-same-path-note.md"),
    )
    .unwrap();
    assert_eq!(raw_after, raw_before, "raw must not be rewritten on conflict");
}

#[test]
fn archive_note_document_rejects_empty_body_without_writing() {
    let (_sandbox, repo_root) = setup_corpus();
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    let index_before: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();

    let err = archive_note_document(&repo_root, "   \n  ", &note_opts_with_ts("202607101435"))
        .expect_err("empty body");
    assert!(!err.is_empty());

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert_eq!(index_after, index_before);
    assert!(!corpus.join("raw/inbox/notes").exists() || {
        fs::read_dir(corpus.join("raw/inbox/notes"))
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
    });
}
