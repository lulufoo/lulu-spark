use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::mcp_host::catalog::groups::notes::{
    create_note_from_content, create_note_from_source,
};
use crate::mcp_host::catalog::groups::todo;
use crate::services::todo_task::MIGRATION_GATE_FILE;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = "# Title\n\n---\n\n正文 Body.\n";

fn plant_todo_gate() {
    let root = paths::todo_tasks_dir().expect("todo dir");
    fs::create_dir_all(&root).expect("todo root");
    fs::write(root.join(MIGRATION_GATE_FILE), b"ok\n").expect("gate");
}

fn setup_notes_layout(sandbox: &TestSandbox) {
    let notes = sandbox.workbench_root().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("raw");
    fs::create_dir_all(notes.join("digest")).expect("digest");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
}

fn with_host_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    setup_notes_layout(&sandbox);
    plant_todo_gate();
    f(&sandbox);
}

fn stage_source(sandbox: &TestSandbox, content: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let path = dir.join("source.md");
    fs::write(&path, content).expect("write stage");
    path.canonicalize().expect("canon")
}

fn channel_records(channel: &str) -> Vec<Value> {
    let Ok(path) = paths::message_center_path() else {
        return Vec::new();
    };
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    value
        .get("records")
        .and_then(|v| v.as_array())
        .map(|records| {
            records
                .iter()
                .filter(|record| record.get("channel").and_then(|c| c.as_str()) == Some(channel))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn occupy_message_center_path_as_dir() {
    let path = paths::message_center_path().expect("message_center_path");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("persist parent");
    }
    fs::create_dir_all(&path).expect("occupy persist path as directory");
}

fn invoke_todo(args: Value) -> Value {
    (todo::build("create_todo_task", "workbench")
        .expect("create_todo_task")
        .invoke)(&args)
}

fn assert_note_ok(value: &Value) {
    assert!(value.get("error").is_none(), "unexpected error: {value}");
    assert_eq!(value.get("ok"), Some(&json!(true)), "expected ok: {value}");
}

fn assert_todo_created(value: &Value) {
    assert!(value.get("error").is_none(), "unexpected error: {value}");
    assert_eq!(
        value.get("_status"),
        Some(&json!(201)),
        "expected 201: {value}"
    );
}

fn content_note_args() -> Value {
    json!({
        "content": "AC1 内容笔记正文",
        "title": "AC1 invoke_from_content",
        "digest": "never",
    })
}

fn source_note_args(sandbox: &TestSandbox) -> Value {
    json!({
        "source_path": stage_source(sandbox, SAMPLE_DOC).to_str().unwrap(),
        "title": "AC1 invoke_from_source",
        "digest": "never",
    })
}

fn todo_args(title: &str) -> Value {
    json!({ "title": title, "todo_md": "## Body\n\ntext" })
}

/// Normal: MCP `create_note` `invoke_from_content` success writes a `notes` record.
#[test]
fn create_note_invoke_from_content_success_records_notes_channel() {
    with_host_sandbox(|_| {
        let before = channel_records("notes").len();
        let result = create_note_from_content(&content_note_args());
        assert_note_ok(&result);
        let notes = channel_records("notes");
        assert_eq!(notes.len(), before + 1);
        assert_eq!(
            notes.last().and_then(|r| r.get("channel")),
            Some(&json!("notes"))
        );
    });
}

/// Normal: MCP `create_note` `invoke_from_source` success writes a `notes` record.
#[test]
fn create_note_invoke_from_source_success_records_notes_channel() {
    with_host_sandbox(|sandbox| {
        let before = channel_records("notes").len();
        let result = create_note_from_source(&source_note_args(sandbox));
        assert_note_ok(&result);
        let notes = channel_records("notes");
        assert_eq!(notes.len(), before + 1);
        assert_eq!(
            notes.last().and_then(|r| r.get("channel")),
            Some(&json!("notes"))
        );
    });
}

/// Normal: MCP `create_todo_task` `invoke` success writes a `todos` record.
#[test]
fn create_todo_task_invoke_success_records_todos_channel() {
    with_host_sandbox(|_| {
        let before = channel_records("todos").len();
        let result = invoke_todo(todo_args("AC1 create_todo_task"));
        assert_todo_created(&result);
        let todos = channel_records("todos");
        assert_eq!(todos.len(), before + 1);
        assert_eq!(
            todos.last().and_then(|r| r.get("channel")),
            Some(&json!("todos"))
        );
    });
}

/// Boundary: failed MCP writes do not produce message-center records.
#[test]
fn failed_create_note_and_todo_do_not_produce() {
    with_host_sandbox(|_| {
        let notes_before = channel_records("notes").len();
        let todos_before = channel_records("todos").len();

        let note = create_note_from_content(&json!({
            "title": "Missing content",
            "digest": "never",
        }));
        assert!(note.get("error").is_some(), "{note}");
        assert_ne!(note.get("ok"), Some(&json!(true)));
        assert_eq!(channel_records("notes").len(), notes_before);

        let source = create_note_from_source(&json!({
            "title": "Missing source_path",
            "digest": "never",
        }));
        assert!(source.get("error").is_some(), "{source}");
        assert_ne!(source.get("ok"), Some(&json!(true)));
        assert_eq!(channel_records("notes").len(), notes_before);

        let todo = invoke_todo(json!({ "todo_md": "## Body\n\ntext" }));
        assert!(todo.get("error").is_some(), "{todo}");
        assert_ne!(todo.get("_status"), Some(&json!(201)));
        assert_eq!(channel_records("todos").len(), todos_before);
    });
}

/// Exception: message-center failure does not change a successful MCP response.
#[test]
fn message_center_failure_keeps_successful_mcp_response() {
    with_host_sandbox(|sandbox| {
        occupy_message_center_path_as_dir();

        let note = create_note_from_content(&content_note_args());
        assert_note_ok(&note);
        assert!(channel_records("notes").is_empty());

        let source = create_note_from_source(&source_note_args(sandbox));
        assert_note_ok(&source);
        assert!(channel_records("notes").is_empty());

        let todo = invoke_todo(todo_args("AC1 keep MCP success"));
        assert_todo_created(&todo);
        assert!(channel_records("todos").is_empty());
    });
}
