use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::commands::write::create_note_json;
use crate::config::paths;
use crate::mcp_host::catalog::groups::document::{
    create_doc_from_content, create_doc_from_source, produce_create_doc_if_ok,
    produce_update_doc_if_ok, update_doc_from_content, update_doc_from_source,
};
use crate::mcp_host::catalog::{build, group_for_migrated_api};
use crate::services::message_center::last_changed_envelope;
use crate::services::docs::create_jot;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = "# Title\n\n---\n\n正文 Body.\n";

fn setup_notes_layout(sandbox: &TestSandbox) {
    let notes = sandbox.data_dir().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("raw");
    fs::create_dir_all(notes.join("digest")).expect("digest");
}

fn with_host_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    setup_notes_layout(&sandbox);
    f(&sandbox);
}

fn stage_source(sandbox: &TestSandbox, content: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let path = dir.join("source.md");
    fs::write(&path, content).expect("write stage");
    path.canonicalize().expect("canon")
}

fn channel_record_count(channel: &str) -> usize {
    let Ok(path) = paths::message_center_path() else {
        return 0;
    };
    if !path.is_file() {
        return 0;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return 0;
    };
    value
        .get("records")
        .and_then(|v| v.as_array())
        .map(|records| {
            records
                .iter()
                .filter(|record| record.get("channel").and_then(|c| c.as_str()) == Some(channel))
                .count()
        })
        .unwrap_or(0)
}

fn occupy_message_center_path_as_dir() {
    let path = paths::message_center_path().expect("message_center_path");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("persist parent");
    }
    fs::create_dir_all(&path).expect("occupy persist path as directory");
}

fn invoke_notes(name: &str, args: Value) -> Value {
    let group = group_for_migrated_api(name).expect(name);
    (build(group, name, "spark").expect(name).invoke)(&args)
}

fn assert_note_ok(value: &Value) {
    assert!(value.get("error").is_none(), "unexpected error: {value}");
    assert_eq!(value.get("ok"), Some(&json!(true)), "expected ok: {value}");
}
fn notes_identity_params(result: &Value) -> Value {
    let mut params = serde_json::Map::new();
    if let Some(value) = result.get("id") {
        params.insert("archive_id".to_string(), value.clone());
    }
    Value::Object(params)
}

fn assert_mcp_write_hides_internal_paths(result: &Value) {
    for key in [
        "common_path",
        "raw_path",
        "digest_path",
        "extra_paths",
        "asset_paths",
        "created_at",
    ] {
        assert!(result.get(key).is_none(), "{key} leaked: {result}");
    }
}

fn assert_produced_envelope(business: &str, action: &str, params: Value) {
    let e = last_changed_envelope().expect("envelope");
    assert_eq!(e.business.as_str(), business);
    assert_eq!(e.action.as_str(), action);
    let mut got = e.params.clone();
    let hop_id = got
        .as_object_mut()
        .expect("params object")
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_string))
        .expect("hop id");
    assert!(
        crate::services::os_notify_trace::parse_trace_id(&hop_id).is_some(),
        "{hop_id}"
    );
    if let Some(archive_id) = params.get("archive_id").and_then(|v| v.as_str()) {
        assert_ne!(hop_id, archive_id);
    }
    assert!(e.params.get("trace_id").is_none(), "{e:?}");
    assert_eq!(&got, &params);
    assert!(e.params.get("scheme").is_none(), "{e:?}");
}

fn assert_notes_produced(action: &str, result: &Value, before: usize) {
    assert_note_ok(result);
    assert_mcp_write_hides_internal_paths(result);
    let params = notes_identity_params(result);
    let envelope = last_changed_envelope().expect("envelope");
    assert!(
        params.get("archive_id").is_some() && params.get("common_path").is_none(),
        "{result} envelope={envelope:?}"
    );
    let note_id = result.get("id").and_then(|v| v.as_str()).expect("note id");
    let hop_id = last_changed_envelope()
        .expect("envelope")
        .params
        .get("id")
        .and_then(|v| v.as_str())
        .expect("hop id")
        .to_string();
    assert_ne!(hop_id, note_id);
    assert_eq!(channel_record_count("docs"), before + 1);
    assert_produced_envelope("docs", action, params);
}

fn content_note_args() -> Value {
    json!({
        "workspace_id": crate::test_support::default_workspace().id,
        "content": "MCP 产出笔记正文",
        "title": "MCP produce content",
        "digest": "never",
        "source_type": "summary",
    })
}

fn source_note_args(sandbox: &TestSandbox) -> Value {
    json!({
        "workspace_id": crate::test_support::default_workspace().id,
        "source_path": stage_source(sandbox, SAMPLE_DOC).to_str().unwrap(),
        "title": "MCP produce source",
        "digest": "never",
        "source_type": "summary",
    })
}

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn assert_file_has_no_message_center(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(!text.contains("message_center") && !text.contains("::produce("), "{} must not wire message center", path.display());
}

fn assert_file_has_no_message_center_produce(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(!text.contains("message_center::produce") && !text.contains("::produce("), "{} must not wire message center produce", path.display());
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn create_doc_from_content_success_adds_notes_record() {
    with_host_sandbox(|_| {
        let before = channel_record_count("docs");
        let result = create_doc_from_content(&content_note_args());
        assert_notes_produced("create", &result, before);
    });
}

#[test]
fn create_doc_from_source_success_adds_notes_record() {
    with_host_sandbox(|sandbox| {
        let before = channel_record_count("docs");
        let result = create_doc_from_source(&source_note_args(sandbox));
        assert_notes_produced("create", &result, before);
    });
}

#[test]
fn update_doc_from_source_success_adds_notes_record() {
    with_host_sandbox(|sandbox| {
        let created = create_doc_from_source(&source_note_args(sandbox));
        assert_note_ok(&created);
        let before = channel_record_count("docs");
        let result = update_doc_from_source(&json!({
            "id": created["id"].as_str().unwrap(),
            "source_path": stage_source(sandbox, "# New\n\n---\n\n改过的正文\n").to_str().unwrap(),
            "digest": "never",
        }));
        assert_notes_produced("update", &result, before);
    });
}

#[test]
fn update_doc_from_content_success_adds_notes_record() {
    with_host_sandbox(|_| {
        let created = create_doc_from_content(&content_note_args());
        assert_note_ok(&created);
        let before = channel_record_count("docs");
        let result = update_doc_from_content(&json!({
            "id": created["id"].as_str().unwrap(),
            "content": "# New\n\n---\n\n手机改正文\n",
            "digest": "never",
        }));
        assert_notes_produced("update", &result, before);
    });
}

#[test]
fn other_note_and_tauri_writes_are_not_wired() {
    let mut files = Vec::new();
    for group in ["global", "workspace", "project", "document", "read_later"] {
        collect_rs_files(&src(&format!("mcp_host/catalog/groups/{group}")), &mut files);
    }
    for path in files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "create_doc.rs" || name == "update_doc.rs" {
            continue;
        }
        assert_file_has_no_message_center(&path);
    }
    assert_file_has_no_message_center_produce(&src("commands/write.rs"));
    assert_file_has_no_message_center(&src("services/docs/jot.rs"));

    with_host_sandbox(|sandbox| {
        let notes_before = channel_record_count("docs");
        let env_before = last_changed_envelope();
        let category = invoke_notes(
            "create_project",
            json!({ "project_name": "No produce category" }),
        );
        assert!(category.get("error").is_none(), "{category}");
        assert_eq!(channel_record_count("docs"), notes_before);

        let tauri_note = create_note_json(json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": stage_source(sandbox, SAMPLE_DOC).to_str().unwrap(),
            "title": "Tauri create_note",
            "digest": "never",
            "source_type": "summary",
        }))
        .expect("tauri create_note");
        assert_note_ok(&tauri_note);
        assert_eq!(channel_record_count("docs"), notes_before);

        let jot = create_jot(
            Some(&crate::test_support::default_workspace().id),
            None,
            "随手记一行",
            &Default::default(),
        )
        .expect("create_jot");
        assert_note_ok(&jot);
        assert_eq!(channel_record_count("docs"), notes_before);
        assert_eq!(last_changed_envelope(), env_before);
    });
}

#[test]
fn failed_business_write_does_not_produce() {
    with_host_sandbox(|_| {
        let notes_before = channel_record_count("docs");
        let env_before = last_changed_envelope();
        let note = create_doc_from_content(&json!({
            "title": "Missing content",
            "digest": "never",
        }));
        assert!(note.get("error").is_some(), "{note}");
        assert_ne!(note.get("ok"), Some(&json!(true)));
        let updated = update_doc_from_content(&json!({
            "id": "missing-note",
            "content": "# Missing\n\n---\n\n不会写入\n",
            "digest": "never",
        }));
        assert!(updated.get("error").is_some() || updated.get("ok") != Some(&json!(true)));
        assert_eq!(channel_record_count("docs"), notes_before);
        assert_eq!(last_changed_envelope(), env_before);
    });
}

#[test]
fn produce_notes_if_ok_keeps_timing_when_identity_fields_missing() {
    with_host_sandbox(|_| {
        let before = channel_record_count("docs");
        let cases = [
            (produce_create_doc_if_ok as fn(&Value), json!({"ok": true}), "create", json!({})),
            (produce_create_doc_if_ok, json!({"ok": true, "id": "only-id"}), "create", json!({"archive_id": "only-id"})),
            (produce_create_doc_if_ok, json!({"ok": true, "common_path": "only/path.md"}), "create", json!({})),
            (produce_update_doc_if_ok, json!({"ok": true, "id": "u1"}), "update", json!({"archive_id": "u1"})),
        ];
        let produced = cases.len();
        for (produce_if_ok, result, action, params) in cases {
            produce_if_ok(&result);
            assert_produced_envelope("docs", action, params);
        }
        produce_create_doc_if_ok(&json!({"ok": false, "id": "nope"}));
        produce_create_doc_if_ok(&json!({"error": "boom", "ok": true, "id": "nope"}));
        produce_update_doc_if_ok(&json!({"error": "boom", "id": "nope"}));
        assert_eq!(channel_record_count("docs"), before + produced);
    });
}

#[test]
fn message_center_failure_does_not_change_successful_mcp_response() {
    with_host_sandbox(|sandbox| {
        occupy_message_center_path_as_dir();
        let note = create_doc_from_content(&content_note_args());
        assert_note_ok(&note);
        assert_eq!(channel_record_count("docs"), 0);
        let source = create_doc_from_source(&source_note_args(sandbox));
        assert_note_ok(&source);
        assert_eq!(channel_record_count("docs"), 0);
    });
}
