use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::notes::{
    create_note_content, create_note, create_jot, synthesize_jot_document, JotCreateOpts,
};
use crate::services::todo_task::{create_master_with_subs, get_by_id};
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

fn stage_source(sandbox: &TestSandbox, name: &str, content: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, content).expect("write stage");
    p.canonicalize().expect("canon")
}

fn add_project(id: &str) {
    crate::services::notes::create_notes_category(id, id, "").expect("notes category");
}

fn path_payload(sandbox: &TestSandbox, content: &str, mut base: serde_json::Value) -> serde_json::Value {
    let path = stage_source(sandbox, "source.md", content);
    let obj = base.as_object_mut().expect("object");
    obj.insert("source_path".to_string(), json!(path.to_str().unwrap()));
    if !obj.contains_key("title") {
        obj.insert("title".to_string(), json!("Test Title"));
    }
    if !obj.contains_key("digest") {
        obj.insert("digest".to_string(), json!("never"));
    }
    obj.remove("document");
    base
}

fn filename_has_ts_rand(common_path: &str) -> bool {
    let file = common_path.rsplit('/').next().unwrap_or("");
    file.len() >= 19
        && file[..12].chars().all(|c| c.is_ascii_digit())
        && file.as_bytes().get(12) == Some(&b'-')
        && file[13..19]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
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
    let common_path = v["common_path"].as_str().expect("common_path");
    assert!(common_path.starts_with("inbox/notes/"), "{common_path}");
    assert!(common_path.ends_with("-source.md"), "{common_path}");
    assert!(filename_has_ts_rand(common_path), "{common_path}");
    let raw = notes.join("raw").join(common_path);
    assert!(raw.is_file());
    let raw_text = fs::read_to_string(&raw).unwrap();
    assert!(raw_text.starts_with("# Test Title\n"));
    assert!(!raw_text.contains("[digest]("));
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert!(index["entries"][id]["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
}

#[test]
fn create_note_second_write_gets_new_filename() {
    let (sandbox, repo_root) = setup_notes();
    let payload = path_payload(&sandbox, SAMPLE_DOC, json!({}));
    let first = create_note(&repo_root, &payload);
    let second = create_note(&repo_root, &payload);
    assert_eq!(first.get("ok"), Some(&json!(true)));
    assert_eq!(second.get("ok"), Some(&json!(true)));
    assert_ne!(first["common_path"], second["common_path"]);
}

#[test]
fn create_note_always_writes_digest_and_updates_layers() {
    let (sandbox, repo_root) = setup_notes();
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview";
    let v = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({
                "digest": "always",
                "digest_body": digest_body,
            }),
        ),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "create_note failed: {v}");
    let id = v["id"].as_str().unwrap();
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let common_path = v["common_path"].as_str().unwrap();
    let digest = notes.join("digest").join(common_path);
    assert!(digest.is_file());
    assert_eq!(v["digest_path"], json!(format!("digest/{common_path}")));
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(
        index["entries"][id]["layers"],
        json!(["raw", "digest"])
    );
}

#[test]
fn create_note_requires_digest() {
    let (sandbox, repo_root) = setup_notes();
    let path = stage_source(&sandbox, "source.md", SAMPLE_DOC);
    let v = create_note(
        &repo_root,
        &json!({
            "source_path": path.to_str().unwrap(),
            "title": "Test Title",
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("digest"), "{v}");
}

#[test]
fn create_note_always_requires_digest_body() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, SAMPLE_DOC, json!({ "digest": "always" })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"].as_str().unwrap_or("").contains("digest_body"),
        "{v}"
    );
}

#[test]
fn create_note_auto_skips_short_summary() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, SAMPLE_DOC, json!({ "digest": "auto" })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    assert!(v.get("digest_path").is_none());
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let common_path = v["common_path"].as_str().unwrap();
    assert!(!notes.join("digest").join(common_path).is_file());
}

#[test]
fn create_note_auto_requires_digest_body_when_ad0_applies() {
    let (sandbox, repo_root) = setup_notes();
    let long = format!("# T\n\n---\n\n{}", "x".repeat(200));
    let v = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            &long,
            json!({ "digest": "auto", "source_type": "summary" }),
        ),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"].as_str().unwrap_or("").contains("digest_body"),
        "{v}"
    );
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
    add_project("learning-ai-agent");
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "title": "Interview Title",
            "project": "learning-ai-agent",
            "theme": "waymo-interview",
            "created_at": "202606191700",
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let common_path = v["common_path"].as_str().unwrap();
    assert!(common_path.starts_with("learning-ai-agent/waymo-interview/202606191700-"));
    let zh_path = format!("{}-zh.md", common_path.trim_end_matches(".md"));
    assert!(notes.join(format!("raw/{zh_path}")).is_file());
    assert_eq!(v["extra_paths"], json!([format!("raw/{zh_path}")]));
    let id = v["id"].as_str().unwrap();
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["entries"][id]["translations"]["zh"], json!(zh_path));
}

#[test]
fn create_note_zh_from_source_path() {
    let (sandbox, repo_root) = setup_notes();
    add_project("learning-ai-agent");
    let zh_file = stage_source(&sandbox, "zh.md", THEME_LINE_ZH);
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, THEME_LINE_DOC, json!({
            "title": "Interview Title",
            "project": "learning-ai-agent",
            "theme": "waymo-interview",
            "created_at": "202606191700",
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "source_path": zh_file.to_str().unwrap() }]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let common_path = v["common_path"].as_str().unwrap();
    let zh_path = format!("raw/{}-zh.md", common_path.trim_end_matches(".md"));
    assert_eq!(v["extra_paths"], json!([zh_path]));
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
    let stem = v["common_path"].as_str().unwrap().trim_end_matches(".md");
    assert_eq!(
        index["entries"][id]["translations"]["zh"],
        json!(format!("{stem}-zh.md"))
    );
    assert_eq!(
        index["entries"][id]["translations"]["fr"],
        json!(format!("{stem}-fr.md"))
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
fn create_note_ignores_task_ids_and_does_not_link_todo() {
    let (sandbox, repo_root) = setup_notes();
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

    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    assert!(index["entries"][archive_id].get("task_ref").is_none());

    let task = get_by_id(master_id).expect("todo");
    let sub = &task["sub_tasks"][0];
    assert_eq!(sub["status"], "incomplete");
    assert!(sub["linked_archive_ids"].as_array().unwrap().is_empty());
}

fn jot_opts_with_ts(ts: &str) -> JotCreateOpts {
    JotCreateOpts {
        ts: Some(ts.to_string()),
        ..JotCreateOpts::default()
    }
}

#[test]
fn synthesize_jot_document_builds_shell_without_nav() {
    let body = "First line title\n\nMore body content.";
    let doc = synthesize_jot_document(body, &jot_opts_with_ts("202607101430"))
        .expect("synthesize");
    assert!(doc.starts_with("# First line title\n"), "H1 from first line: {doc}");
    assert!(
        doc.contains("> 创建时间：2026年7月10日 14:30"),
        "created_at line: {doc}"
    );
    assert!(!doc.contains("[digest]("), "no digest nav: {doc}");
    assert!(
        doc.contains("---\n\nFirst line title\n\nMore body content."),
        "body preserved after separator: {doc}"
    );
}

#[test]
fn synthesize_jot_document_empty_first_line_falls_back_to_ts() {
    let body = "\n\nBody without a title line.";
    let doc = synthesize_jot_document(body, &jot_opts_with_ts("202607101431"))
        .expect("synthesize");
    assert!(
        doc.starts_with("# 202607101431\n"),
        "H1 falls back to timestamp: {doc}"
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
    assert!(common_path.starts_with("inbox/notes/202607101433-"), "{common_path}");
    assert!(common_path.ends_with(".md"), "{common_path}");
    assert!(filename_has_ts_rand(common_path), "{common_path}");

    let notes = crate::config::meili_env::notes_root_path(&repo_root);
    let raw = notes.join("raw").join(common_path);
    assert!(raw.is_file(), "raw must exist at {raw:?}");
    let raw_text = fs::read_to_string(&raw).expect("read raw");
    assert!(raw_text.starts_with("# Quick capture\n"));
    assert!(raw_text.contains("Details here."));
    assert!(!raw_text.contains("[digest]("));

    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
    let entry = &index["entries"][id];
    assert_eq!(entry["source_type"], json!("jot"));
    assert_eq!(entry["common_path"], json!(common_path));
    assert!(entry["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
}

#[test]
fn create_jot_second_write_gets_new_filename() {
    let (_sandbox, repo_root) = setup_notes();
    let opts = jot_opts_with_ts("202607101434");
    let body = "Same path note";
    let first = create_jot(&repo_root, body, &opts).expect("first create");
    let second = create_jot(&repo_root, body, &opts).expect("second create");
    assert_eq!(first.get("ok"), Some(&json!(true)));
    assert_eq!(second.get("ok"), Some(&json!(true)));
    assert_ne!(first["common_path"], second["common_path"]);
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
fn create_note_content_writes_from_markdown_body() {
    let (_sandbox, repo_root) = setup_notes();
    let v = create_note_content(
        &repo_root,
        &json!({
            "content": SAMPLE_DOC,
            "title": "Test Title",
            "source_type": "summary",
            "digest": "never",
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "create_note_content failed: {v}");
    assert_eq!(v["id"].as_str().expect("id").len(), 32);
    let common_path = v["common_path"].as_str().expect("common_path");
    assert!(common_path.starts_with("inbox/notes/"), "{common_path}");
    assert!(common_path.ends_with(".md"), "{common_path}");
    assert!(!common_path.ends_with("-source.md"), "{common_path}");
    assert!(filename_has_ts_rand(common_path), "{common_path}");
}

#[test]
fn create_note_content_requires_title() {
    let (_sandbox, repo_root) = setup_notes();
    let v = create_note_content(
        &repo_root,
        &json!({
            "content": "body only",
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("title"), "{v}");
}

#[test]
fn create_note_rejects_unknown_project() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({ "source_type": "summary", "project": "not-a-category" }),
        ),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("Unknown project"), "{v}");
}

#[test]
fn create_note_rejects_unknown_source_type() {
    let (sandbox, repo_root) = setup_notes();
    let v = create_note(
        &repo_root,
        &path_payload(&sandbox, SAMPLE_DOC, json!({ "source_type": "podcast" })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_content_rejects_source_path_and_empty() {
    let (_sandbox, repo_root) = setup_notes();
    let with_path = create_note_content(
        &repo_root,
        &json!({
            "content": SAMPLE_DOC,
            "source_path": "/tmp/x.md",
        }),
    );
    assert_eq!(with_path.get("_status"), Some(&json!(400)));
    assert!(
        with_path["error"]
            .as_str()
            .unwrap_or("")
            .contains("content"),
        "{with_path}"
    );
    let empty = create_note_content(&repo_root, &json!({ "content": "   " }));
    assert_eq!(empty.get("_status"), Some(&json!(400)));
    let missing = create_note_content(&repo_root, &json!({ "source_type": "summary" }));
    assert_eq!(missing.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_requires_title() {
    let (sandbox, repo_root) = setup_notes();
    let path = stage_source(&sandbox, "source.md", SAMPLE_DOC);
    let v = create_note(
        &repo_root,
        &json!({
            "source_path": path.to_str().unwrap(),
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("title"), "{v}");
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
            "title": "Test Title",
            "source_type": "summary",
            "digest": "never",
        }),
    );
    let status = v.get("_status").and_then(|s| s.as_u64()).unwrap_or(0);
    assert!(
        status == 403 || status == 404,
        "expected 403/404, got {v}"
    );
}
