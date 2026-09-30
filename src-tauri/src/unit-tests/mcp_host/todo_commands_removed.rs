//! t6 red gate: todo command module, registration, and todo tool-name assertions are gone.
//! These checks compile whether or not the command module still exists.

use std::fs;
use std::path::PathBuf;

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn text(rel: &str) -> String {
    fs::read_to_string(src(rel)).unwrap_or_else(|err| panic!("read {rel}: {err}"))
}

#[test]
fn todo_command_directory_is_gone() {
    let dir = src("commands/todo_task");
    assert!(
        !dir.exists(),
        "commands/todo_task must be deleted"
    );
}

#[test]
fn commands_and_lib_do_not_declare_or_register_todo() {
    let forbidden = [
        ("commands/mod.rs", "pub mod todo_task"),
        ("lib.rs", "commands::todo_task::"),
    ];
    for (rel, marker) in forbidden {
        let body = text(rel);
        assert!(
            !body.contains(marker),
            "{rel} still contains `{marker}`"
        );
    }
}

#[test]
fn todo_contract_test_file_is_gone() {
    let path = src("unit-tests/mcp_host/todo_contract.rs");
    assert!(
        !path.exists(),
        "unit-tests/mcp_host/todo_contract.rs must be deleted"
    );
}

#[test]
fn mcp_host_tests_do_not_assert_todo_tool_names() {
    let body = text("unit-tests/mcp_host.rs");
    assert!(
        !body.contains("const TODO_TOOLS"),
        "unit-tests/mcp_host.rs must not keep TODO_TOOLS"
    );
    for marker in [
        "\"create_todo_task\"",
        "\"list_todo_tasks\"",
        "\"list_todo_categories\"",
        "\"add_todo_sub\"",
    ] {
        assert!(
            !body.contains(marker),
            "unit-tests/mcp_host.rs still asserts todo tool name {marker}"
        );
    }
}
