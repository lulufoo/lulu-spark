use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::commands::write::create_note_json;
use crate::config::paths;
use crate::mcp_host::catalog::groups::notes::{
    self, create_note_from_content, create_note_from_source,
};
use crate::mcp_host::catalog::groups::todo;
use crate::services::notes::create_jot;
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

fn invoke_todo(name: &str, args: Value) -> Value {
    (todo::build(name, "workbench").expect(name).invoke)(&args)
}

fn invoke_notes(name: &str, args: Value) -> Value {
    (notes::build(name, "workbench").expect(name).invoke)(&args)
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

fn todo_args(title: &str) -> Value {
    json!({ "title": title, "todo_md": "## Body\n\ntext" })
}

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn assert_file_has_no_message_center(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        !text.contains("message_center") && !text.contains("::produce("),
        "{} must not wire message center",
        path.display()
    );
}

fn assert_file_has_no_message_center_produce(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        !text.contains("message_center::produce") && !text.contains("::produce("),
        "{} must not wire message center produce",
        path.display()
    );
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
        assert_note_ok(&result);
        assert_eq!(channel_record_count("notes"), before + 1);
    });
}

#[test]
fn create_note_from_source_success_adds_notes_record() {
    with_host_sandbox(|sandbox| {
        let before = channel_record_count("notes");
        let result = create_note_from_source(&source_note_args(sandbox));
        assert_note_ok(&result);
        assert_eq!(channel_record_count("notes"), before + 1);
    });
}

#[test]
fn create_todo_task_success_adds_todos_record() {
    with_host_sandbox(|_| {
        let before = channel_record_count("todos");
        let result = invoke_todo("create_todo_task", todo_args("MCP produce todo"));
        assert_todo_created(&result);
        assert_eq!(channel_record_count("todos"), before + 1);
    });
}

#[test]
fn other_note_todo_and_tauri_writes_are_not_wired() {
    let mut files = Vec::new();
    collect_rs_files(&src("mcp_host/catalog/groups/notes"), &mut files);
    collect_rs_files(&src("mcp_host/catalog/groups/todo"), &mut files);
    for path in files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "create_note.rs" || name == "create_todo_task.rs" {
            continue;
        }
        assert_file_has_no_message_center(&path);
    }
    assert_file_has_no_message_center_produce(&src("commands/write.rs"));
    assert_file_has_no_message_center(&src("services/notes/jot.rs"));

    with_host_sandbox(|sandbox| {
        let notes_before = channel_record_count("notes");
        let category = invoke_notes(
            "create_notes_category",
            json!({ "title": "No produce category" }),
        );
        assert!(category.get("error").is_none(), "{category}");
        assert_eq!(channel_record_count("notes"), notes_before);

        let created = invoke_todo("create_todo_task", todo_args("Parent for sub"));
        assert_todo_created(&created);
        let after_create = channel_record_count("todos");
        let master_id = created["master_task_id"].as_str().expect("master_task_id");
        let added = invoke_todo(
            "add_todo_sub",
            json!({
                "master_task_id": master_id,
                "title": "Sub",
                "content": "required body"
            }),
        );
        assert_eq!(added.get("_status"), Some(&json!(201)), "{added}");
        assert_eq!(channel_record_count("todos"), after_create);

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
    });
}

#[test]
fn failed_business_write_does_not_produce() {
    with_host_sandbox(|_| {
        let notes_before = channel_record_count("notes");
        let todos_before = channel_record_count("todos");

        let note = create_note_from_content(&json!({
            "title": "Missing content",
            "digest": "never",
        }));
        assert!(note.get("error").is_some(), "{note}");
        assert_ne!(note.get("ok"), Some(&json!(true)));
        assert_eq!(channel_record_count("notes"), notes_before);

        let todo = invoke_todo("create_todo_task", json!({ "todo_md": "## Body\n\ntext" }));
        assert!(todo.get("error").is_some(), "{todo}");
        assert_ne!(todo.get("_status"), Some(&json!(201)));
        assert_eq!(channel_record_count("todos"), todos_before);
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

        let todo = invoke_todo("create_todo_task", todo_args("MCP keep success"));
        assert_todo_created(&todo);
        assert_eq!(channel_record_count("todos"), 0);
    });
}
