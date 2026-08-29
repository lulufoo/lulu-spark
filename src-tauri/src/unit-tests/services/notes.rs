use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::services::archive_parse::parse_archive_document;
use crate::services::notes::{
    create_note_digest, create_note, create_jot, synthesize_jot_document,
    JotCreateOpts,
};
use crate::services::todo_task::{
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

fn setup_notes() -> (TestSandbox, std::path::PathBuf) {
    let sandbox = TestSandbox::new();
    let notes = sandbox.workbench_root().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
    let repo_root = sandbox.config_dir().to_path_buf();
    (sandbox, repo_root)
}

fn read_v2_index(wb: &Path) -> serde_json::Value {
    serde_json::from_str(
        &fs::read_to_string(wb.join("todo_tasks").join("index.json")).expect("index.json"),
    )
    .expect("parse index.json")
}

fn stage_source(sandbox: &TestSandbox, name: &str, content: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, content).expect("write stage");
    p.canonicalize().expect("canon")
}

fn path_payload(sandbox: &TestSandbox, content: &str, mut base: serde_json::Value) -> serde_json::Value {
    let path = stage_source(sandbox, "source.md", content);
    let obj = base.as_object_mut().expect("object");
    obj.insert("source_path".to_string(), json!(path.to_str().unwrap()));
    obj.remove("document");
    base
}


#[test]
fn create_note_writes_raw_and_index() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, SAMPLE_DOC, json!({ "source_type": "summary" })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "create_note failed: {v}");
    let id = v["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let raw = notes.join("raw/inbox/test-topic/202606191430-test-slug.md");
    assert!(raw.is_file());
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert!(index["entries"][id]["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
}

#[test]
fn create_note_conflict_returns_409() {
    let (sandbox, repo_root) = setup_notes();
    let payload = path_payload(&sandbox, SAMPLE_DOC, json!({}));
    assert_eq!(create_note(&repo_root, &payload).get("ok"), Some(&json!(true)));
    let v = create_note(&repo_root, &payload);
    assert_eq!(v.get("_status"), Some(&json!(409)));
}

#[test]
fn create_note_digest_writes_digest_and_updates_layers() {
    let (sandbox, repo_root) = setup_notes();
    let created = create_note(&repo_root, &path_payload(&sandbox, SAMPLE_DOC, json!({})));
    let id = created["id"].as_str().unwrap();
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview";
    let v = create_note_digest(
        &repo_root,
        &json!({ "id": id, "digest": digest_body }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)));
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let digest = notes.join("digest/inbox/test-topic/202606191430-test-slug.md");
    assert!(digest.is_file());
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert!(index["entries"][id]["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("digest")));
}

#[test]
fn create_note_digest_force_overwrites_existing() {
    let (sandbox, repo_root) = setup_notes();
    let created = create_note(&repo_root, &path_payload(&sandbox, SAMPLE_DOC, json!({})));
    let id = created["id"].as_str().unwrap();
    let digest_body = "# Test — 摘要\n\n## 概述\n\nv1";
    create_note_digest(&repo_root, &json!({ "id": id, "digest": digest_body }));
    let blocked = create_note_digest(
        &repo_root,
        &json!({ "id": id, "digest": "# v2\n\n## 概述\n\nblocked" }),
    );
    assert_eq!(blocked.get("_status"), Some(&json!(409)));
    let ok = create_note_digest(
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
fn create_note_theme_line_with_zh_translation() {
    let (sandbox, repo_root) = setup_notes();
    let zh_path = "learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md";
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    assert!(notes.join(format!("raw/{zh_path}")).is_file());
    assert_eq!(
        v["extra_paths"],
        json!([format!("raw/{zh_path}")])
    );
    let id = v["id"].as_str().unwrap();
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["entries"][id]["translations"]["zh"], json!(zh_path));
}

#[test]
fn create_note_zh_from_source_path() {
    let (sandbox, repo_root) = setup_notes();
    let zh_file = stage_source(&sandbox, "zh.md", THEME_LINE_ZH);
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "source_path": zh_file.to_str().unwrap() }]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    assert_eq!(
        v["extra_paths"],
        json!(["raw/learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md"])
    );
}

#[test]
fn create_note_rejects_zh_stub() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "content": "# 题\n\n> 创建时间：2026年6月19日 17:00\n\n---\n\nSEE_FILE\n" }]
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("stub"), "{v}");
}

#[test]
fn create_note_rejects_zh_too_short() {
    let (sandbox, repo_root) = setup_notes();
    let long_en = format!(
        "{}\n{}\nHost: hello\n",
        THEME_LINE_DOC.trim_end(),
        "word ".repeat(80)
    );
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, &long_en, json!({
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("too short"), "{v}");
}

#[test]
fn create_note_rejects_zh_content_and_source_path() {
    let (sandbox, repo_root) = setup_notes();
    let zh_file = stage_source(&sandbox, "zh-both.md", THEME_LINE_ZH);
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "theme-line",
            "translations": [{
                "lang": "zh",
                "content": THEME_LINE_ZH,
                "source_path": zh_file.to_str().unwrap()
            }]
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("not both"), "{v}");
}

#[test]
fn create_note_multi_lang_translations() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "theme-line",
            "translations": [
                { "lang": "zh", "content": THEME_LINE_ZH },
                { "lang": "fr", "content": "# Titre\n\n> 创建时间：2026年6月19日 17:00\n\n---\n\nbonjour\n" }
            ]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let id = v["id"].as_str().unwrap();
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(
        index["entries"][id]["translations"]["zh"],
        json!("learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md")
    );
    assert_eq!(
        index["entries"][id]["translations"]["fr"],
        json!("learning-ai-agent/waymo-interview/202606191700-waymo-interview-fr.md")
    );
}

#[test]
fn create_note_rejects_legacy_extra_documents() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "theme-line",
            "extra_documents": [{
                "rel": "raw/learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md",
                "content": THEME_LINE_ZH
            }]
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    let err = v["error"].as_str().unwrap_or("");
    assert!(err.contains("translations"), "error should mention translations: {v}");
}

#[test]
fn create_note_rejects_legacy_index_extra() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "index_extra": { "translations": { "zh": "x" } }
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_rejects_bad_or_duplicate_lang() {
    let (sandbox, repo_root) = setup_notes();
    let bad = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "translations": [{ "lang": "ZH", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(bad.get("_status"), Some(&json!(400)));

    let dup = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "translations": [
                { "lang": "zh", "content": THEME_LINE_ZH },
                { "lang": "zh", "content": THEME_LINE_ZH }
            ]
        })),
    );
    assert_eq!(dup.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_with_task_ref_completes_sub_in_sandbox() {
    let (sandbox, repo_root) = setup_notes();
    let wb = crate::config::meili_env::workbench_root_path(&repo_root);
    let notes = wb.join("notes");
    let created = create_master_with_subs("Archive link", Some(&["Sub"])).expect("todo");
    let master_id = created["master_task_id"].as_str().unwrap();
    let sub_id = created["sub_task_id"].as_str().unwrap();

    let v = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        ),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "archive failed: {v}");
    let archive_id = v["id"].as_str().expect("id");

    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    let entry = &index["entries"][archive_id];
    assert_eq!(entry["task_ref"]["master_task_id"], master_id);
    assert_eq!(entry["task_ref"]["sub_task_id"], sub_id);

    let task = get_by_id(master_id).expect("todo");
    let sub = &task["sub_tasks"][0];
    assert_eq!(sub["status"], "complete");
    assert_eq!(
        sub["linked_archive_ids"].as_array().unwrap()[0],
        archive_id
    );

    let index = read_v2_index(&wb);
    assert_eq!(index["version"], 2);
    assert!(index["tasks"].get(master_id).is_some());
    let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
    assert!(task_dir.join("sub_tasks.json").is_file());
}

#[test]
fn create_note_todo_task_fail_dual_store_rollback() {
    let (sandbox, repo_root) = setup_notes();
    let wb = crate::config::meili_env::workbench_root_path(&repo_root);
    let notes = wb.join("notes");
    let created = create_master_with_subs("A2 rollback", Some(&["Sub"])).expect("todo");
    let master_id = created["master_task_id"].as_str().unwrap();
    let sub_id = created["sub_task_id"].as_str().unwrap();

    let index_before: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();

    test_set_fail_complete_sub(true);
    let result = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        ),
    );

    assert_ne!(result.get("ok"), Some(&json!(true)), "expected todo_task failure: {result}");
    assert_eq!(result.get("_status"), Some(&json!(500)));

    let raw = notes.join("raw/inbox/test-topic/202606191430-test-slug.md");
    assert!(!raw.exists(), "orphan markdown must be removed after rollback");

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(
        index_after, index_before,
        "index must restore to pre-request snapshot"
    );

    let task = get_by_id(master_id).expect("todo");
    assert_eq!(task["sub_tasks"][0]["status"], "incomplete");
    assert_eq!(
        task["sub_tasks"][0]["linked_archive_ids"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

}

#[test]
fn create_note_index_snapshot_restore_on_todo_task_fail() {
    let (sandbox, repo_root) = setup_notes();
    let wb = crate::config::meili_env::workbench_root_path(&repo_root);
    let notes = wb.join("notes");
    let created = create_master_with_subs("Index snapshot", Some(&["Sub"])).expect("todo");
    let master_id = created["master_task_id"].as_str().unwrap();
    let sub_id = created["sub_task_id"].as_str().unwrap();

    let index_path = notes.join("index.json");
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
    let result = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        ),
    );
    assert_ne!(result.get("ok"), Some(&json!(true)));

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    assert_eq!(
        index_after, index_before,
        "rollback must restore full entries map including pre-existing keys"
    );

}

#[test]
fn create_note_link_fail_notes_rollback_plan_stays_complete() {
    let (sandbox, repo_root) = setup_notes();
    let wb = crate::config::meili_env::workbench_root_path(&repo_root);
    let notes = wb.join("notes");
    let created = create_master_with_subs("Link fail", Some(&["Sub"])).expect("todo");
    let master_id = created["master_task_id"].as_str().unwrap();
    let sub_id = created["sub_task_id"].as_str().unwrap();

    let index_before: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();

    test_set_fail_link_archive(true);
    let result = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({
                "source_type": "summary",
                "master_task_id": master_id,
                "sub_task_id": sub_id,
            }),
        ),
    );

    assert_ne!(result.get("ok"), Some(&json!(true)), "expected link failure: {result}");
    assert_eq!(result.get("_status"), Some(&json!(500)));

    let raw = notes.join("raw/inbox/test-topic/202606191430-test-slug.md");
    assert!(!raw.exists(), "notes raw must rollback on link failure");

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(index_after, index_before, "notes index must restore to pre-request snapshot");

    let task = get_by_id(master_id).expect("todo");
    let sub = &task["sub_tasks"][0];
    assert_eq!(sub["status"], "complete", "complete_sub succeeded before link failure");
    assert_eq!(
        sub["linked_archive_ids"].as_array().unwrap().len(),
        0,
        "link_archive must not append on failure"
    );
}

fn jot_opts_with_ts(ts: &str) -> JotCreateOpts {
    JotCreateOpts {
        ts: Some(ts.to_string()),
        ..JotCreateOpts::default()
    }
}

#[test]
fn synthesize_jot_document_builds_parseable_shell() {
    let body = "First line title\n\nMore body content.";
    let doc = synthesize_jot_document(body, &jot_opts_with_ts("202607101430"))
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
fn synthesize_jot_document_empty_first_line_falls_back_to_ts_slug() {
    let body = "\n\nBody without a title line.";
    let doc = synthesize_jot_document(body, &jot_opts_with_ts("202607101431"))
        .expect("synthesize");
    let parsed = parse_archive_document(&doc).expect("parse");
    assert_eq!(
        parsed.common_path,
        "inbox/notes/202607101431-jot.md"
    );
    assert!(
        doc.starts_with("# 202607101431-jot\n") || doc.starts_with("# jot\n"),
        "H1 falls back to timestamp/slug: {doc}"
    );
}

#[test]
fn synthesize_jot_document_defaults_topic_inbox() {
    let doc = synthesize_jot_document("Hello note", &jot_opts_with_ts("202607101432"))
        .expect("synthesize");
    let parsed = parse_archive_document(&doc).expect("parse");
    assert!(
        parsed.common_path.starts_with("inbox/"),
        "topic must default to inbox: {}",
        parsed.common_path
    );
}

#[test]
fn create_jot_writes_raw_index_with_source_type_jot() {
    let (_sandbox, repo_root) = setup_notes();
    let body = "Quick capture\n\nDetails here.";
    let v = create_jot(&repo_root, body, &jot_opts_with_ts("202607101433"))
        .expect("create_jot");
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let id = v["id"].as_str().expect("id");
    let common_path = v["common_path"].as_str().expect("common_path");
    assert_eq!(common_path, "inbox/notes/202607101433-quick-capture.md");

    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let raw = notes.join("raw").join(common_path);
    assert!(raw.is_file(), "raw must exist at {raw:?}");
    let raw_text = fs::read_to_string(&raw).expect("read raw");
    assert!(raw_text.contains("Quick capture"));
    parse_archive_document(&raw_text).expect("stored doc must remain parseable");

    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    let entry = &index["entries"][id];
    assert_eq!(entry["source_type"], json!("jot"));
    assert_eq!(entry["common_path"], json!(common_path));
    assert!(entry["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
    assert!(
        !notes.join("annotations").exists()
            || fs::read_dir(notes.join("annotations"))
                .map(|mut d| d.next().is_none())
                .unwrap_or(true),
        "must not write Annotation path"
    );
}

#[test]
fn create_jot_conflict_does_not_mutate_history() {
    let (_sandbox, repo_root) = setup_notes();
    let opts = jot_opts_with_ts("202607101434");
    let body = "Same path note";
    let first = create_jot(&repo_root, body, &opts).expect("first create");
    assert_eq!(first.get("ok"), Some(&json!(true)));
    let id = first["id"].as_str().unwrap().to_string();
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let index_before: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    let raw_before = fs::read_to_string(
        notes.join("raw/inbox/notes/202607101434-same-path-note.md"),
    )
    .unwrap();

    let err = create_jot(&repo_root, body, &opts).expect_err("409 conflict");
    assert!(
        err.contains("409") || err.to_lowercase().contains("already exists"),
        "expected conflict error: {err}"
    );

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(index_after, index_before, "history Entry must not change on conflict");
    assert_eq!(
        index_after["entries"][&id]["source_type"],
        json!("jot")
    );
    let raw_after = fs::read_to_string(
        notes.join("raw/inbox/notes/202607101434-same-path-note.md"),
    )
    .unwrap();
    assert_eq!(raw_after, raw_before, "raw must not be rewritten on conflict");
}

#[test]
fn create_jot_rejects_empty_body_without_writing() {
    let (_sandbox, repo_root) = setup_notes();
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let index_before: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();

    let err = create_jot(&repo_root, "   \n  ", &jot_opts_with_ts("202607101435"))
        .expect_err("empty body");
    assert!(!err.is_empty());

    let index_after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(index_after, index_before);
    assert!(!notes.join("raw/inbox/notes").exists() || {
        fs::read_dir(notes.join("raw/inbox/notes"))
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
    });
}

#[test]
fn create_note_rejects_document_field() {
    let (_sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &json!({
            "document": SAMPLE_DOC,
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"]
            .as_str()
            .unwrap_or("")
            .contains("source_path"),
        "{v}"
    );
}

#[test]
fn create_note_rejects_disallowed_source_path() {
    let (_sandbox, repo_root) = setup_notes();
    // Absolute path outside allow-roots (and typically missing).
    let outside = PathBuf::from("/var/empty/lulu-workbench-archive-forbid.md");
    let v = create_note(
        &repo_root,
        &json!({
            "source_path": outside.to_str().unwrap(),
            "source_type": "summary",
        }),
    );
    let status = v.get("_status").and_then(|s| s.as_u64()).unwrap_or(0);
    assert!(
        status == 403 || status == 404,
        "expected 403/404, got {v}"
    );
}
