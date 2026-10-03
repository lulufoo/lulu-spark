use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::commands::write::create_note_json;
use crate::config::paths;
use crate::mcp_host::catalog::groups::notes::{
    self, create_note_from_content, create_note_from_source, produce_create_note_if_ok,
    produce_update_note_if_ok, update_note_from_content, update_note_from_source,
};
use crate::services::message_center::last_changed_envelope;
use crate::services::notes::create_jot;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = "# Title\n\n---\n\n正文 Body.\n";

fn setup_notes_layout(sandbox: &TestSandbox) {
    let notes = sandbox.workbench_root().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("raw");
    fs::create_dir_all(notes.join("digest")).expect("digest");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
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
    (notes::build(name, "workbench").expect(name).invoke)(&args)
}

fn assert_note_ok(value: &Value) {
    assert!(value.get("error").is_none(), "unexpected error: {value}");
    assert_eq!(value.get("ok"), Some(&json!(true)), "expected ok: {value}");
}
fn notes_identity_params(result: &Value) -> Value {
    let mut params = serde_json::Map::new();
    for key in ["id", "common_path"] {
        if let Some(value) = result.get(key) { params.insert(key.to_string(), value.clone()); }
    }
    Value::Object(params)
}

fn assert_produced_envelope(business: &str, action: &str, params: Value) {
    let e = last_changed_envelope().expect("envelope");
    assert_eq!(e.business.as_str(), business);
    assert_eq!(e.action.as_str(), action);
    let mut got = e.params.clone();
    let tid = got
        .as_object_mut()
        .expect("params object")
        .remove("trace_id")
        .and_then(|v| v.as_str().map(str::to_string))
        .expect("trace_id");
    assert!(
        crate::services::os_notify_trace::parse_trace_id(&tid).is_some(),
        "{tid}"
    );
    assert_eq!(&got, &params);
    assert!(e.params.get("scheme").is_none(), "{e:?}");
}

fn assert_notes_produced(action: &str, result: &Value, before: usize) {
    assert_note_ok(result);
    let params = notes_identity_params(result);
    assert!(params.get("id").is_some() && params.get("common_path").is_some(), "{result}");
    assert_eq!(channel_record_count("notes"), before + 1);
    assert_produced_envelope("notes", action, params);
}

fn content_note_args() -> Value {
    json!({
        "content": "MCP 产出笔记正文",
        "title": "MCP produce content",
        "digest": "never",
    })
}

fn source_note_args(sandbox: &TestSandbox) -> Value {
    json!({
        "source_path": stage_source(sandbox, SAMPLE_DOC).to_str().unwrap(),
        "title": "MCP produce source",
        "digest": "never",
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
fn create_note_from_content_success_adds_notes_record() {
    with_host_sandbox(|_| {
        let before = channel_record_count("notes");
        let result = create_note_from_content(&content_note_args());
        assert_notes_produced("create", &result, before);
    });
}

#[test]
fn create_note_from_source_success_adds_notes_record() {
    with_host_sandbox(|sandbox| {
        let before = channel_record_count("notes");
        let result = create_note_from_source(&source_note_args(sandbox));
        assert_notes_produced("create", &result, before);
    });
}

#[test]
fn update_note_from_source_success_adds_notes_record() {
    with_host_sandbox(|sandbox| {
        let created = create_note_from_source(&source_note_args(sandbox));
        assert_note_ok(&created);
        let before = channel_record_count("notes");
        let result = update_note_from_source(&json!({
            "id": created["id"].as_str().unwrap(),
            "source_path": stage_source(sandbox, "# New\n\n---\n\n改过的正文\n").to_str().unwrap(),
            "digest": "never",
        }));
        assert_notes_produced("update", &result, before);
    });
}

#[test]
fn update_note_from_content_success_adds_notes_record() {
    with_host_sandbox(|_| {
        let created = create_note_from_content(&content_note_args());
        assert_note_ok(&created);
        let before = channel_record_count("notes");
        let result = update_note_from_content(&json!({
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
    collect_rs_files(&src("mcp_host/catalog/groups/notes"), &mut files);
    for path in files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "create_note.rs" || name == "update_note.rs" {
            continue;
        }
        assert_file_has_no_message_center(&path);
    }
    assert_file_has_no_message_center_produce(&src("commands/write.rs"));
    assert_file_has_no_message_center(&src("services/notes/jot.rs"));

    with_host_sandbox(|sandbox| {
        let notes_before = channel_record_count("notes");
        let env_before = last_changed_envelope();
        let category = invoke_notes(
            "create_notes_category",
            json!({ "title": "No produce category" }),
        );
        assert!(category.get("error").is_none(), "{category}");
        assert_eq!(channel_record_count("notes"), notes_before);

        let tauri_note = create_note_json(json!({
            "source_path": stage_source(sandbox, SAMPLE_DOC).to_str().unwrap(),
            "title": "Tauri create_note",
            "digest": "never",
        }))
        .expect("tauri create_note");
        assert_note_ok(&tauri_note);
        assert_eq!(channel_record_count("notes"), notes_before);

        let jot = create_jot(
            &paths::repo_root().expect("repo_root"),
            "随手记一行",
            &Default::default(),
        )
        .expect("create_jot");
        assert_note_ok(&jot);
        assert_eq!(channel_record_count("notes"), notes_before);
        assert_eq!(last_changed_envelope(), env_before);
    });
}

#[test]
fn failed_business_write_does_not_produce() {
    with_host_sandbox(|_| {
        let notes_before = channel_record_count("notes");
        let env_before = last_changed_envelope();
        let note = create_note_from_content(&json!({
            "title": "Missing content",
            "digest": "never",
        }));
        assert!(note.get("error").is_some(), "{note}");
        assert_ne!(note.get("ok"), Some(&json!(true)));
        let updated = update_note_from_content(&json!({
            "id": "missing-note",
            "content": "# Missing\n\n---\n\n不会写入\n",
            "digest": "never",
        }));
        assert!(updated.get("error").is_some() || updated.get("ok") != Some(&json!(true)));
        assert_eq!(channel_record_count("notes"), notes_before);
        assert_eq!(last_changed_envelope(), env_before);
    });
}

#[test]
fn produce_notes_if_ok_keeps_timing_when_identity_fields_missing() {
    with_host_sandbox(|_| {
        let before = channel_record_count("notes");
        let cases = [
            (produce_create_note_if_ok as fn(&Value), json!({"ok": true}), "create", json!({})),
            (produce_create_note_if_ok, json!({"ok": true, "id": "only-id"}), "create", json!({"id": "only-id"})),
            (produce_create_note_if_ok, json!({"ok": true, "common_path": "only/path.md"}), "create", json!({"common_path": "only/path.md"})),
            (produce_update_note_if_ok, json!({"ok": true, "id": "u1"}), "update", json!({"id": "u1"})),
        ];
        let produced = cases.len();
        for (produce_if_ok, result, action, params) in cases {
            produce_if_ok(&result);
            assert_produced_envelope("notes", action, params);
        }
        produce_create_note_if_ok(&json!({"ok": false, "id": "nope"}));
        produce_create_note_if_ok(&json!({"error": "boom", "ok": true, "id": "nope"}));
        produce_update_note_if_ok(&json!({"error": "boom", "id": "nope"}));
        assert_eq!(channel_record_count("notes"), before + produced);
    });
}

#[test]
fn message_center_failure_does_not_change_successful_mcp_response() {
    with_host_sandbox(|sandbox| {
        occupy_message_center_path_as_dir();
        let note = create_note_from_content(&content_note_args());
        assert_note_ok(&note);
        assert_eq!(channel_record_count("notes"), 0);
        let source = create_note_from_source(&source_note_args(sandbox));
        assert_note_ok(&source);
        assert_eq!(channel_record_count("notes"), 0);
    });
}
