use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::docs::{create_doc, create_doc_content, update_doc, update_doc_content};
use crate::services::docs::{create_jot, synthesize_jot_document, JotCreateOpts};
use crate::test_support::{block_note_writes, read_note_entries, TestSandbox};

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

摘要正文，enough content.
"#;

fn setup_notes() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = sandbox.data_dir().join("workspaces").join("default");
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    sandbox
}

fn stage_source(sandbox: &TestSandbox, name: &str, content: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, content).expect("write stage");
    p.canonicalize().expect("canon")
}

fn add_project(title: &str) -> String {
    let v = crate::services::docs::create_project(&json!({ "title": title }));
    assert_eq!(v["ok"], json!(true), "{v}");
    v["project"]["id"].as_str().expect("id").to_string()
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
    if !obj.contains_key("source_type") {
        obj.insert("source_type".to_string(), json!("summary"));
    }
    obj.remove("document");
    base
}

fn default_workspace_dir() -> PathBuf {
    crate::config::paths::workspaces_root().expect("workspaces root").join("default")
}

fn project_id(folder: &str) -> String {
    crate::services::spark_data::get_project_by_folder(
        &crate::services::spark_data::open().expect("db"),
        "default",
        folder,
    )
    .expect("lookup")
    .expect("project")
    .id
}

fn note_row(id: &str) -> crate::services::spark_data::DocRow {
    crate::services::spark_data::find_doc(&crate::services::spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .doc
}

fn note_disk(notes: &std::path::Path, layer: &str, id: &str) -> PathBuf {
    let conn = crate::services::spark_data::open().expect("db");
    let note = crate::services::spark_data::find_doc(&conn, id)
        .expect("get")
        .expect("row")
        .doc;
    crate::services::docs::doc_file_path(notes, layer, &note.project_id, &note.raw_path).expect("path")
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
fn create_note_removes_its_files_when_the_table_write_fails() {
    let sandbox = setup_notes();
    block_note_writes("INSERT");
    let v = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({ "source_type": "summary" })),
    );
    assert_eq!(v["_status"], 500, "{v}");
    let raw = sandbox.data_dir().join("workspaces/default/raw");
    let leftovers: Vec<_> = walk_files(&raw);
    assert!(leftovers.is_empty(), "files are written first and rolled back: {leftovers:?}");
    assert_eq!(read_note_entries()["entries"], json!({}));
}

fn walk_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk_files(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn create_note_writes_raw_and_index() {
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({ "source_type": "summary" })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "create_note failed: {v}");
    let id = v["id"].as_str().expect("id");
    assert!(crate::services::spark_data::is_valid_doc_id(id) && id.starts_with("default-"), "{id}");
    let notes = default_workspace_dir();
    let rel = note_row(id).raw_path;
    assert!(rel.starts_with("notes/"), "{rel}");
    assert!(rel.ends_with("-source.md"), "{rel}");
    assert!(filename_has_ts_rand(&rel), "{rel}");
    let raw = note_disk(&notes, "raw", id);
    assert!(raw.is_file());
    let raw_text = fs::read_to_string(&raw).unwrap();
    assert_eq!(raw_text, SAMPLE_DOC);
    let index: serde_json::Value =
        read_note_entries();
    assert_eq!(index["entries"][id]["title"], json!("Test Title"));
    assert!(index["entries"][id]["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
    let row = note_row(id);
    assert_eq!(row.project_id, project_id("inbox"));
    assert_eq!(row.raw_path, rel);
}

#[test]
fn create_note_second_write_gets_new_filename() {
    let sandbox = setup_notes();
    let payload = path_payload(&sandbox, SAMPLE_DOC, json!({}));
    let first = create_doc(&payload);
    let second = create_doc(&payload);
    assert_eq!(first.get("ok"), Some(&json!(true)));
    assert_eq!(second.get("ok"), Some(&json!(true)));
    assert_ne!(
        note_row(first["id"].as_str().unwrap()).raw_path,
        note_row(second["id"].as_str().unwrap()).raw_path
    );
}

#[test]
fn create_note_always_writes_digest_and_updates_layers() {
    let sandbox = setup_notes();
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview";
    let v = create_doc(&path_payload(
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
    let notes = default_workspace_dir();
    let rel = note_row(id).raw_path;
    let digest = note_disk(&notes, "digest", id);
    assert!(digest.is_file());
    assert_eq!(v["digest_path"], json!(rel));
    let index: serde_json::Value =
        read_note_entries();
    assert_eq!(
        index["entries"][id]["layers"],
        json!(["raw", "digest"])
    );
}

#[test]
fn create_note_requires_digest() {
    let sandbox = setup_notes();
    let path = stage_source(&sandbox, "source.md", SAMPLE_DOC);
    let v = create_doc(&json!({
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
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({ "digest": "always" })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"].as_str().unwrap_or("").contains("digest_body"),
        "{v}"
    );
}

#[test]
fn create_note_auto_skips_short_summary() {
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({ "digest": "auto" })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    assert!(v.get("digest_path").is_none());
    let notes = default_workspace_dir();
    let id = v["id"].as_str().unwrap();
    assert!(!note_disk(&notes, "digest", id).is_file());
}

#[test]
fn create_note_auto_requires_digest_body_when_ad0_applies() {
    let sandbox = setup_notes();
    let long = format!("# T\n\n---\n\n{}", "x".repeat(200));
    let v = create_doc(&path_payload(
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
> 时长：约 30 分钟 · 发布：2026-01-01
> 导航：[digest](../../../digest/learning-ai-agent/waymo-interview/202606191700-waymo-interview.md)
> 原文：[Video](https://example.com/watch)

---

## 主题一
主持人：你好
"#;

#[test]
fn create_note_theme_line_with_zh_translation() {
    let sandbox = setup_notes();
    let project = add_project("learning-ai-agent");
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "title": "Interview Title",
            "project": project,
            "folder": "waymo-interview",
            "created_at": "202606191700",
            "source_type": "video",
            "translations": [{ "lang": "zh", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let notes = default_workspace_dir();
    let id = v["id"].as_str().unwrap();
    let rel = note_row(id).raw_path;
    assert!(rel.starts_with("waymo-interview/202606191700-"), "{rel}");
    let zh_rel = format!("{}-zh.md", rel.trim_end_matches(".md"));
    assert!(notes.join("raw").join(&project).join(&zh_rel).is_file());
    assert_eq!(v["extra_paths"], json!([format!("raw/{zh_rel}")]));
    let index: serde_json::Value =
        read_note_entries();
    assert_eq!(index["entries"][id]["translations"]["zh"], json!(zh_rel));
    let row = note_row(id);
    assert_eq!(row.project_id, project);
    assert_eq!(row.raw_path, rel);
}

#[test]
fn create_note_rejects_the_renamed_theme_field() {
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "title": "Interview Title",
            "theme": "waymo-interview",
            "source_type": "video"
        })),
    );
    assert_eq!(v["_status"], json!(400), "{v}");
    assert!(v["error"].as_str().unwrap().contains("renamed to folder"), "{v}");
}

#[test]
fn create_note_zh_from_source_path() {
    let sandbox = setup_notes();
    let project = add_project("learning-ai-agent");
    let zh_file = stage_source(&sandbox, "zh.md", THEME_LINE_ZH);
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "title": "Interview Title",
            "project": project,
            "folder": "waymo-interview",
            "created_at": "202606191700",
            "source_type": "video",
            "translations": [{ "lang": "zh", "source_path": zh_file.to_str().unwrap() }]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let rel = note_row(v["id"].as_str().unwrap()).raw_path;
    let zh_path = format!("raw/{}-zh.md", rel.trim_end_matches(".md"));
    assert_eq!(v["extra_paths"], json!([zh_path]));
}

#[test]
fn create_note_rejects_zh_stub() {
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "video",
            "translations": [{ "lang": "zh", "content": "# 题\n\n> 创建时间：2026年6月19日 17:00\n\n---\n\nSEE_FILE\n" }]
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("stub"), "{v}");
}

#[test]
fn create_note_rejects_zh_too_short() {
    let sandbox = setup_notes();
    let long_en = format!("{}\n{}\n", THEME_LINE_DOC.trim_end(), "word ".repeat(400));
    let v = create_doc(&path_payload(&sandbox, &long_en, json!({
            "source_type": "video",
            "translations": [{ "lang": "zh", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("too short"), "{v}");
}

#[test]
fn create_note_rejects_zh_content_and_source_path() {
    let sandbox = setup_notes();
    let zh_file = stage_source(&sandbox, "zh-both.md", THEME_LINE_ZH);
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "video",
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
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "video",
            "translations": [
                { "lang": "zh", "content": THEME_LINE_ZH },
                { "lang": "fr", "content": "# Titre\n\n> 创建时间：2026年6月19日 17:00\n\n---\n\nbonjour\n" }
            ]
        })),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let id = v["id"].as_str().unwrap();
    let index: serde_json::Value = read_note_entries();
    let stem = note_row(id).raw_path.trim_end_matches(".md").to_string();
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
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "source_type": "video",
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
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "index_extra": { "translations": { "zh": "x" } }
        })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_rejects_bad_or_duplicate_lang() {
    let sandbox = setup_notes();
    let bad = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
            "translations": [{ "lang": "ZH", "content": THEME_LINE_ZH }]
        })),
    );
    assert_eq!(bad.get("_status"), Some(&json!(400)));

    let dup = create_doc(&path_payload(&sandbox, THEME_LINE_DOC, json!({
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
    let sandbox = setup_notes();
    // Fake ids — todo_task service removed; create_note must still ignore task_ref fields.
    let master_id = "bind-Archive-link";
    let sub_id = "sub-Sub";

    let v = create_doc(&path_payload(
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

    let index: serde_json::Value = read_note_entries();
    assert!(index["entries"][archive_id].get("task_ref").is_none());
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
    let _sandbox = setup_notes();
    let body = "快速记录\n\nDetails here.";
    let v = create_jot(None, None, body, &jot_opts_with_ts("202607101433"))
        .expect("create_jot");
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let id = v["id"].as_str().expect("id");
    let rel = note_row(id).raw_path;
    assert!(rel.starts_with("notes/202607101433-"), "{rel}");
    assert!(rel.ends_with(".md"), "{rel}");
    assert!(filename_has_ts_rand(&rel), "{rel}");

    let notes = default_workspace_dir();
    let raw = note_disk(&notes, "raw", id);
    assert!(raw.is_file(), "raw must exist at {raw:?}");
    let raw_text = fs::read_to_string(&raw).expect("read raw");
    assert!(raw_text.starts_with("# 快速记录\n"));
    assert!(raw_text.contains("Details here."));
    assert!(!raw_text.contains("[digest]("));

    let index: serde_json::Value =
        read_note_entries();
    let entry = &index["entries"][id];
    assert_eq!(entry["source_type"], json!("jot"));
    assert_eq!(entry["folder"], json!("inbox"));
    assert!(entry["layers"]
        .as_array()
        .unwrap()
        .contains(&json!("raw")));
}

#[test]
fn create_jot_second_write_gets_new_filename() {
    let _sandbox = setup_notes();
    let opts = jot_opts_with_ts("202607101434");
    let body = "同一路径笔记";
    let first = create_jot(None, None, body, &opts).expect("first create");
    let second = create_jot(None, None, body, &opts).expect("second create");
    assert_eq!(first.get("ok"), Some(&json!(true)));
    assert_eq!(second.get("ok"), Some(&json!(true)));
    assert_ne!(
        note_row(first["id"].as_str().unwrap()).raw_path,
        note_row(second["id"].as_str().unwrap()).raw_path
    );
}

#[test]
fn create_jot_rejects_empty_body_without_writing() {
    let _sandbox = setup_notes();
    let notes = default_workspace_dir();
    let index_before: serde_json::Value =
        read_note_entries();

    let err = create_jot(None, None, "   \n  ", &jot_opts_with_ts("202607101435"))
        .expect_err("empty body");
    assert!(!err.is_empty());

    let index_after: serde_json::Value =
        read_note_entries();
    assert_eq!(index_after, index_before);
    assert!(!notes.join("raw/inbox/notes").exists() || {
        fs::read_dir(notes.join("raw/inbox/notes"))
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
    });
}

#[test]
fn create_note_content_writes_from_markdown_body() {
    let _sandbox = setup_notes();
    let v = create_doc_content(&json!({
            "content": SAMPLE_DOC,
            "title": "Test Title",
            "source_type": "summary",
            "digest": "never",
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "create_note_content failed: {v}");
    let id = v["id"].as_str().expect("id");
    assert!(crate::services::spark_data::is_valid_doc_id(id) && id.starts_with("default-"), "{id}");
    let rel = note_row(id).raw_path;
    assert!(rel.starts_with("notes/"), "{rel}");
    assert!(rel.ends_with(".md"), "{rel}");
    assert!(!rel.ends_with("-source.md"), "{rel}");
    assert!(filename_has_ts_rand(&rel), "{rel}");
}

#[test]
fn create_note_content_requires_title() {
    let _sandbox = setup_notes();
    let v = create_doc_content(&json!({
            "content": "body only",
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("title"), "{v}");
}

#[test]
fn create_note_rejects_unknown_project() {
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(
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
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({ "source_type": "podcast" })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_requires_source_type() {
    let sandbox = setup_notes();
    let mut payload = path_payload(&sandbox, SAMPLE_DOC, json!({}));
    payload.as_object_mut().expect("object").remove("source_type");
    let v = create_doc(&payload);
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"].as_str().unwrap_or("").contains("source_type"),
        "{v}"
    );
}

#[test]
fn create_note_rejects_legacy_theme_line_id() {
    let sandbox = setup_notes();
    let v = create_doc(&path_payload(
        &sandbox,
        SAMPLE_DOC,
        json!({ "source_type": "theme-line" }),
    ));
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_accepts_video_and_dialogue() {
    let sandbox = setup_notes();
    for source_type in ["video", "dialogue", "article", "summary"] {
        let v = create_doc(&path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({ "source_type": source_type }),
        ));
        assert_eq!(v.get("ok"), Some(&json!(true)), "{source_type}: {v}");
        assert_eq!(note_row(v["id"].as_str().unwrap()).source_type.as_deref(), Some(source_type));
    }
}

#[test]
fn create_note_content_rejects_source_path_and_empty() {
    let _sandbox = setup_notes();
    let with_path = create_doc_content(&json!({
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
    let empty = create_doc_content(&json!({ "content": "   " }));
    assert_eq!(empty.get("_status"), Some(&json!(400)));
    let missing = create_doc_content(&json!({ "source_type": "summary" }));
    assert_eq!(missing.get("_status"), Some(&json!(400)));
}

#[test]
fn create_note_requires_title() {
    let sandbox = setup_notes();
    let path = stage_source(&sandbox, "source.md", SAMPLE_DOC);
    let v = create_doc(&json!({
            "source_path": path.to_str().unwrap(),
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(v["error"].as_str().unwrap_or("").contains("title"), "{v}");
}

#[test]
fn create_note_rejects_document_field() {
    let _sandbox = setup_notes();
    let v = create_doc(&json!({
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
fn create_note_missing_source_outside_allow_roots_is_404_not_403() {
    let _sandbox = setup_notes();
    let outside = PathBuf::from("/var/empty/lulu-spark-archive-forbid.md");
    let v = create_doc(&json!({
            "source_path": outside.to_str().unwrap(),
            "title": "Test Title",
            "source_type": "summary",
            "digest": "never",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(404)), "{v}");
}

const FULL_EN_DOC: &str = r#"# English Title

Hello world. This body is English only.
"#;

const FREEFORM_EN: &str = "# Andrew Ng\n\nReflection is the first pattern.\n";
const FREEFORM_ZH: &str = "# Andrew Ng\n\n反思是第一种模式。\n";

#[test]
fn create_note_rejects_full_english_without_zh() {
    let sandbox = setup_notes();
    let notes = default_workspace_dir();
    let v = create_doc(&path_payload(&sandbox, FULL_EN_DOC, json!({ "digest": "never" })),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"]
            .as_str()
            .unwrap_or("")
            .contains("translations.zh"),
        "{v}"
    );
    assert!(!notes.join("raw/inbox").exists() || {
        fs::read_dir(notes.join("raw/inbox"))
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
    });
}

#[test]
fn create_note_writes_source_as_is_with_zh_and_no_separator() {
    let sandbox = setup_notes();
    let zh = stage_source(&sandbox, "zh.md", FREEFORM_ZH);
    let v = create_doc(&path_payload(
            &sandbox,
            FREEFORM_EN,
            json!({
                "digest": "never",
                "translations": [{ "lang": "zh", "source_path": zh.to_str().unwrap() }],
            }),
        ),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    let notes = default_workspace_dir();
    let id = v["id"].as_str().unwrap();
    let raw = fs::read_to_string(note_disk(&notes, "raw", id)).unwrap();
    assert_eq!(raw, FREEFORM_EN);
    let rel = note_row(id).raw_path;
    let zh_path = format!("{}-zh.md", rel.trim_end_matches(".md"));
    let zh_raw = fs::read_to_string(notes.join("raw/inbox").join(zh_path)).unwrap();
    assert_eq!(zh_raw, FREEFORM_ZH);
}

#[test]
fn create_note_content_rejects_full_english_without_zh() {
    let _sandbox = setup_notes();
    let v = create_doc_content(&json!({
            "content": FULL_EN_DOC,
            "title": "English Title",
            "digest": "never",
            "source_type": "summary",
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"]
            .as_str()
            .unwrap_or("")
            .contains("translations.zh"),
        "{v}"
    );
}

fn update_path_payload(
    sandbox: &TestSandbox,
    id: &str,
    content: &str,
    mut base: serde_json::Value,
) -> serde_json::Value {
    let path = stage_source(sandbox, "update.md", content);
    let obj = base.as_object_mut().expect("object");
    obj.insert("id".to_string(), json!(id));
    obj.insert("source_path".to_string(), json!(path.to_str().unwrap()));
    if !obj.contains_key("digest") {
        obj.insert("digest".to_string(), json!("never"));
    }
    base
}

#[test]
fn update_note_overwrites_raw_and_keeps_id_path() {
    let sandbox = setup_notes();
    let created = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({})),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap().to_string();
    let v = update_doc(&update_path_payload(&sandbox, &id, "# New\n\n---\n\n改过的正文\n", json!({})),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    assert_eq!(v["id"], json!(id));
    assert!(v.get("common_path").is_none());
    let notes = default_workspace_dir();
    let raw = fs::read_to_string(note_disk(&notes, "raw", &id)).unwrap();
    assert!(raw.contains("改过的正文"), "{raw}");
    assert!(!raw.contains("enough content"), "{raw}");
}

#[test]
fn update_note_never_leaves_existing_digest() {
    let sandbox = setup_notes();
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview";
    let created = create_doc(&path_payload(
            &sandbox,
            SAMPLE_DOC,
            json!({ "digest": "always", "digest_body": digest_body }),
        ),
    );
    let id = created["id"].as_str().unwrap();
    let notes = default_workspace_dir();
    let digest = note_disk(&notes, "digest", id);
    let v = update_doc(&update_path_payload(&sandbox, id, "# New\n\n---\n\n只改正文\n", json!({})),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    assert!(v.get("digest_path").is_none());
    assert_eq!(fs::read_to_string(&digest).unwrap(), digest_body);
}

#[test]
fn update_note_always_overwrites_digest_and_adds_layer() {
    let sandbox = setup_notes();
    let created = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({})),
    );
    let id = created["id"].as_str().unwrap().to_string();
    let digest_body = "# 新摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\nupdated";
    let v = update_doc(&update_path_payload(
            &sandbox,
            &id,
            "# New\n\n---\n\n改过的正文\n",
            json!({ "digest": "always", "digest_body": digest_body }),
        ),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    let rel = note_row(&id).raw_path;
    assert_eq!(v["digest_path"], json!(rel));
    let notes = default_workspace_dir();
    assert_eq!(
        fs::read_to_string(note_disk(&notes, "digest", &id)).unwrap(),
        digest_body
    );
    let index: serde_json::Value =
        read_note_entries();
    assert_eq!(index["entries"][&id]["layers"], json!(["raw", "digest"]));
    assert_eq!(index["entries"][&id]["folder"], json!("inbox"));
}

#[test]
fn update_note_requires_digest_and_rejects_title() {
    let sandbox = setup_notes();
    let created = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({})),
    );
    let id = created["id"].as_str().unwrap();
    let path = stage_source(&sandbox, "update.md", "# New\n\n---\n\n改过的正文\n");
    let missing = update_doc(&json!({
            "id": id,
            "source_path": path.to_str().unwrap(),
        }),
    );
    assert_eq!(missing.get("_status"), Some(&json!(400)));
    assert!(
        missing["error"].as_str().unwrap_or("").contains("digest"),
        "{missing}"
    );
    let titled = update_doc(&update_path_payload(
            &sandbox,
            id,
            "# New\n\n---\n\n改过的正文\n",
            json!({ "title": "Nope" }),
        ),
    );
    assert_eq!(titled.get("_status"), Some(&json!(400)));
    assert!(
        titled["error"].as_str().unwrap_or("").contains("title"),
        "{titled}"
    );
}

#[test]
fn update_note_rejects_bad_or_missing_id() {
    let sandbox = setup_notes();
    let path = stage_source(&sandbox, "update.md", "# New\n\n---\n\n改过的正文\n");
    let missing = update_doc(&json!({
            "source_path": path.to_str().unwrap(),
            "digest": "never",
        }),
    );
    assert_eq!(missing.get("_status"), Some(&json!(400)));
    assert!(
        missing["error"].as_str().unwrap_or("").contains("id"),
        "{missing}"
    );
    let bad = update_doc(&update_path_payload(&sandbox, "not-an-id", "# New\n\n---\n\n改过的正文\n", json!({})),
    );
    assert_eq!(bad.get("_status"), Some(&json!(400)));
    let missing_entry = update_doc(&update_path_payload(
            &sandbox,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "# New\n\n---\n\n改过的正文\n",
            json!({}),
        ),
    );
    assert_eq!(missing_entry.get("_status"), Some(&json!(404)));
}

#[test]
fn update_note_content_overwrites_and_rejects_source_path() {
    let _sandbox = setup_notes();
    let created = create_doc_content(&json!({
            "content": SAMPLE_DOC,
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let v = update_doc_content(&json!({
            "id": id,
            "content": "# New\n\n---\n\n手机改正文\n",
            "digest": "never",
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    let notes = default_workspace_dir();
    let raw = fs::read_to_string(note_disk(&notes, "raw", id)).unwrap();
    assert!(raw.contains("手机改正文"), "{raw}");
    let mixed = update_doc_content(&json!({
            "id": id,
            "content": "# x\n",
            "source_path": "/tmp/x.md",
            "digest": "never",
        }),
    );
    assert_eq!(mixed.get("_status"), Some(&json!(400)));
}

#[test]
fn update_note_rejects_full_english_without_zh() {
    let sandbox = setup_notes();
    let created = create_doc(&path_payload(&sandbox, SAMPLE_DOC, json!({})),
    );
    let id = created["id"].as_str().unwrap();
    let v = update_doc(&update_path_payload(&sandbox, id, FULL_EN_DOC, json!({})),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"].as_str().unwrap_or("").contains("translations.zh"),
        "{v}"
    );
}

#[test]
fn create_jot_rejects_full_english_without_zh() {
    let _sandbox = setup_notes();
    let err = create_jot(None, None, "Only English jot body.",
        &jot_opts_with_ts("202607101440"),
    )
    .expect_err("english jot");
    assert!(err.contains("translations.zh"), "{err}");
}

fn jot_dirs(root: &std::path::Path) -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files
}

#[test]
fn create_jot_with_a_project_writes_into_that_project() {
    let _sandbox = setup_notes();
    let plans = add_project("plans");
    let v = create_jot(None, Some(&plans), "放进计划\n\n正文", &jot_opts_with_ts("202607101440"))
        .expect("create_jot");
    assert_eq!(v["ok"], json!(true), "{v}");
    assert_eq!(v["project"], json!(plans));
    let id = v["id"].as_str().expect("id");
    assert!(note_row(id).raw_path.starts_with("notes/202607101440-"));
    let entry = &read_note_entries()["entries"][id];
    assert_eq!(entry["folder"], json!(plans));
    assert_eq!(entry["source_type"], json!("jot"));
    let raw = default_workspace_dir().join("raw").join(&plans).join("notes");
    assert!(jot_dirs(&raw).iter().any(|p| p.extension().is_some_and(|e| e == "md")), "{raw:?}");
}

#[test]
fn create_jot_without_a_project_still_goes_to_inbox() {
    let _sandbox = setup_notes();
    let v = create_jot(None, None, "没有指定项目", &jot_opts_with_ts("202607101441")).expect("create_jot");
    assert_eq!(v["project"], json!("inbox"));
    let id = v["id"].as_str().expect("id");
    assert_eq!(read_note_entries()["entries"][id]["folder"], json!("inbox"));
}

#[test]
fn create_jot_refuses_a_missing_or_disabled_project_and_writes_nothing() {
    let _sandbox = setup_notes();
    let old = add_project("old");
    let disabled = crate::services::docs::disable_project(&json!({ "project": old }));
    assert_eq!(disabled["ok"], json!(true), "{disabled}");
    let before = jot_dirs(&default_workspace_dir());

    for project in ["ghost".to_string(), old] {
        let err = create_jot(None, Some(&project), "不该写入", &jot_opts_with_ts("202607101442"))
            .expect_err(&project);
        assert!(err.starts_with("400:") || err.starts_with("404:") || err.starts_with("409:"), "{project}: {err}");
    }
    assert_eq!(jot_dirs(&default_workspace_dir()), before, "nothing was written");
}

#[test]
fn create_jot_refuses_a_project_of_another_workspace() {
    let _sandbox = setup_notes();
    let other = crate::services::docs::create_workspace(&json!({ "name": "Other" }));
    let other_id = other["workspace"]["id"].as_str().expect("workspace id").to_string();
    let made = crate::services::docs::create_project(
        &json!({ "workspace": other_id, "title": "Theirs" }),
    );
    assert_eq!(made["ok"], json!(true), "{made}");
    let theirs = made["project"]["id"].as_str().expect("project id");
    let before = jot_dirs(&default_workspace_dir());
    let err = create_jot(Some("default"), Some(theirs), "跨工作区", &jot_opts_with_ts("202607101443"))
        .expect_err("another workspace's project");
    assert!(err.starts_with("400:") || err.starts_with("404:"), "{err}");
    assert_eq!(jot_dirs(&default_workspace_dir()), before);
}
