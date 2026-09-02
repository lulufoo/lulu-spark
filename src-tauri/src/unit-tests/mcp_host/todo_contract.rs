use crate::mcp_host::catalog::groups::todo;
use crate::services::todo_task::MIGRATION_GATE_FILE;
use crate::test_support::TestSandbox;
use serde_json::{json, Value};
use std::fs;

fn plant_todo_migration_gate() {
    let root = crate::config::paths::todo_tasks_dir().expect("todo dir");
    fs::create_dir_all(&root).expect("todo root");
    fs::write(root.join(MIGRATION_GATE_FILE), b"ok\n").expect("plant migration gate");
}

fn with_ungated_todo<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    plant_todo_migration_gate();
    f();
}

fn invoke(name: &str, args: Value) -> Value {
    let route = todo::build(name, "workbench").expect(name);
    (route.invoke)(&args)
}

fn schema(name: &str) -> Value {
    todo::build(name, "workbench").expect(name).input_schema
}

#[test]
fn create_todo_task_schema_omits_sub_titles() {
    let schema = schema("create_todo_task");
    assert!(schema["properties"].get("sub_titles").is_none());
}

#[test]
fn add_todo_sub_schema_requires_content() {
    let schema = schema("add_todo_sub");
    let required = schema["required"].as_array().expect("required");
    assert!(required.iter().any(|v| v == "content"));
}

#[test]
fn create_todo_task_ignores_sub_titles_and_yields_empty_tree() {
    with_ungated_todo(|| {
        let created = invoke(
            "create_todo_task",
            json!({
                "title": "MCP create ignore subs",
                "todo_md": "## Body\n\ntext",
                "sub_titles": ["Should be ignored"]
            }),
        );
        assert_eq!(created["_status"], 201);
        let subs = created["task"]["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty(), "leftover sub_titles must not create subs");
    });
}

#[test]
fn add_todo_sub_rejects_missing_empty_and_blank_content() {
    with_ungated_todo(|| {
        let created = invoke(
            "create_todo_task",
            json!({
                "title": "MCP add content gate",
                "todo_md": "## Body\n\ntext"
            }),
        );
        let master_id = created["master_task_id"].as_str().expect("master_task_id");

        let missing = invoke(
            "add_todo_sub",
            json!({
                "master_task_id": master_id,
                "title": "No body"
            }),
        );
        assert_eq!(missing["_status"], 400);
        assert_eq!(missing["error"], "Missing content");

        let empty = invoke(
            "add_todo_sub",
            json!({
                "master_task_id": master_id,
                "title": "Empty body",
                "content": ""
            }),
        );
        assert_eq!(empty["_status"], 400);
        assert_eq!(empty["error"], "Missing content");

        let blank = invoke(
            "add_todo_sub",
            json!({
                "master_task_id": master_id,
                "title": "Blank body",
                "content": "   "
            }),
        );
        assert_eq!(blank["_status"], 400);
        assert_eq!(blank["error"], "Missing content");
    });
}

#[test]
fn add_todo_sub_accepts_nonempty_content() {
    with_ungated_todo(|| {
        let created = invoke(
            "create_todo_task",
            json!({
                "title": "MCP add ok",
                "todo_md": "## Body\n\ntext"
            }),
        );
        let master_id = created["master_task_id"].as_str().expect("master_task_id");
        let added = invoke(
            "add_todo_sub",
            json!({
                "master_task_id": master_id,
                "title": "Has body",
                "content": " required body "
            }),
        );
        assert_eq!(added["_status"], 201);
        assert_eq!(added["task"]["sub_tasks"][0]["content"], "required body");
    });
}

#[test]
fn update_todo_sub_rejects_blank_content_and_omits_keep_title_only() {
    with_ungated_todo(|| {
        let created = invoke(
            "create_todo_task",
            json!({
                "title": "MCP update gate",
                "todo_md": "## Body\n\ntext"
            }),
        );
        let master_id = created["master_task_id"].as_str().expect("master_task_id");
        let added = invoke(
            "add_todo_sub",
            json!({
                "master_task_id": master_id,
                "title": "Old title",
                "content": "keep me"
            }),
        );
        let sub_id = added["sub_task_id"].as_str().expect("sub_task_id");

        let blank = invoke(
            "update_todo_sub",
            json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "Old title",
                "content": "  "
            }),
        );
        assert_eq!(blank["_status"], 400);
        assert_eq!(blank["error"], "Missing content");

        let title_only = invoke(
            "update_todo_sub",
            json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "New title"
            }),
        );
        assert_eq!(title_only["_status"], 200);
        assert_eq!(title_only["task"]["sub_tasks"][0]["title"], "New title");
        assert_eq!(title_only["task"]["sub_tasks"][0]["content"], "keep me");
    });
}
