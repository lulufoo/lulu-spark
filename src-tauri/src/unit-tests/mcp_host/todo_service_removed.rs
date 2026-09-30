//! t8 red gate: todo local service and exclusive tests for it are gone.
//! These checks compile whether or not the service module still exists.

use std::fs;
use std::path::PathBuf;

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn text(rel: &str) -> String {
    fs::read_to_string(src(rel)).unwrap_or_else(|err| panic!("read {rel}: {err}"))
}

#[test]
fn todo_service_directory_is_gone() {
    let dir = src("services/todo_task");
    assert!(
        !dir.exists(),
        "services/todo_task must be deleted"
    );
}

#[test]
fn services_mod_does_not_declare_todo_task() {
    let body = text("services/mod.rs");
    assert!(
        !body.contains("pub mod todo_task"),
        "services/mod.rs still declares pub mod todo_task"
    );
}

#[test]
fn exclusive_todo_service_tests_are_gone() {
    let dir = src("unit-tests/services/todo_task");
    assert!(
        !dir.exists(),
        "unit-tests/services/todo_task must be deleted"
    );
}

#[test]
fn exclusive_todo_command_tests_are_gone() {
    let path = src("unit-tests/commands/todo_task.rs");
    assert!(
        !path.exists(),
        "unit-tests/commands/todo_task.rs must be deleted"
    );
}

#[test]
fn agent_mod_does_not_call_todo_task_service() {
    let body = text("unit-tests/agent/mod.rs");
    assert!(
        !body.contains("use crate::services::todo_task"),
        "unit-tests/agent/mod.rs must not import services::todo_task"
    );
    assert!(
        !body.contains("todo_task::"),
        "unit-tests/agent/mod.rs must not call todo_task::"
    );
}
