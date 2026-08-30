use std::fs;
use std::path::PathBuf;

use crate::services::mcp_channel_tools::set_enabled;
use crate::test_support::TestSandbox;

fn source(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|err| panic!("{rel}: {err}"))
}

#[test]
fn commands_are_declared_and_registered_in_generate_handler() {
    let commands_mod = source("src/commands/mod.rs");
    assert!(
        commands_mod.contains("pub mod mcp_channel_tools"),
        "commands/mod.rs must declare mcp_channel_tools"
    );
    let lib = source("src/lib.rs");
    for name in [
        "commands::mcp_channel_tools::get_mcp_channel_tools",
        "commands::mcp_channel_tools::set_mcp_channel_tools",
    ] {
        assert!(
            lib.contains(name),
            "lib.rs generate_handler must register {name}"
        );
    }
}

#[test]
fn get_and_set_round_trip_in_sandbox() {
    let _sandbox = TestSandbox::new();
    let before = super::get_mcp_channel_tools().expect("get");
    assert!(before["enabled"]["mobile"]
        .as_array()
        .expect("mobile")
        .iter()
        .any(|n| n.as_str() == Some("create_note")));
    set_enabled(
        "mobile",
        vec!["get_all_notes_catalog".into(), "list_todo_tasks".into()],
    )
    .expect("subset");
    let after = super::get_mcp_channel_tools().expect("get after");
    let mobile: Vec<&str> = after["enabled"]["mobile"]
        .as_array()
        .expect("mobile")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(mobile, vec!["get_all_notes_catalog", "list_todo_tasks"]);
    let resp = super::set_mcp_channel_tools(
        "mobile".into(),
        vec!["get_all_notes_catalog".into(), "list_todo_categories".into()],
    )
    .expect("set via command");
    assert_eq!(resp["ok"], true);
}
