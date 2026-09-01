use std::collections::HashSet;
use std::fs;

use crate::config::paths;
use crate::services::mcp_channel_tools::{
    enabled_names, is_enabled, set_enabled, snapshot, MCP_CHANNELS,
};
use crate::services::mcp_protocol_adapter::catalog_groups;
use crate::test_support::TestSandbox;

fn catalog() -> HashSet<String> {
    catalog_groups()
        .into_iter()
        .flat_map(|(_, tools)| tools.into_iter().map(|t| t.name))
        .collect()
}

fn without(name: &str) -> Vec<String> {
    let mut list: Vec<String> = catalog().into_iter().filter(|n| n != name).collect();
    list.sort();
    list
}

#[test]
fn missing_file_enables_full_catalog_on_every_channel() {
    let _sandbox = TestSandbox::new();
    let all = catalog();
    for channel in MCP_CHANNELS {
        let enabled = enabled_names(channel);
        if *channel == "workbench" {
            assert_eq!(enabled, all, "{channel}");
            assert!(is_enabled(channel, "delete_note"), "{channel} delete_note");
        } else {
            assert!(!enabled.contains("delete_note"), "{channel} must hide delete_note");
            assert_eq!(enabled.len(), all.len() - 1, "{channel} catalog minus workbench-only");
        }
        assert!(is_enabled(channel, "create_note"), "{channel} create_note");
    }
    assert!(enabled_names("not-a-channel").is_empty());
    assert!(!is_enabled("not-a-channel", "create_note"));
}

#[test]
fn set_enabled_subset_filters_only_that_channel() {
    let sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_note")).expect("save mobile");
    assert!(!is_enabled("mobile", "create_note"));
    assert!(is_enabled("mobile", "get_all_notes_catalog"));
    assert!(is_enabled("workbench", "create_note"));
    assert!(is_enabled("cursor_ide", "create_note"));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let stored: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).expect("read"))
        .expect("json");
    assert!(stored["channels"].get("workbench").is_none());
    assert!(stored["channels"]["mobile"]
        .as_array()
        .expect("mobile list")
        .iter()
        .all(|n| n.as_str() != Some("create_note")));
}

#[test]
fn set_enabled_all_tools_omits_channel_key() {
    let _sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_note")).expect("subset");
    // Mobile cannot enable workbench-only tools; "all on" = catalog minus those.
    let all_mobile: Vec<String> = {
        let mut names: Vec<String> = catalog()
            .into_iter()
            .filter(|n| n != "delete_note")
            .collect();
        names.sort();
        names
    };
    set_enabled("mobile", all_mobile).expect("restore all");
    assert!(is_enabled("mobile", "create_note"));
    assert!(!is_enabled("mobile", "delete_note"));
    let path = paths::mcp_channel_tools_path().expect("path");
    assert!(
        !path.exists(),
        "all-on should omit every key and drop the file"
    );
}

#[test]
fn set_enabled_rejects_unknown_channel_and_tool() {
    let _sandbox = TestSandbox::new();
    assert!(set_enabled("not-a-channel", vec!["create_note".into()]).is_err());
    assert!(set_enabled("mobile", vec!["not_a_tool".into()]).is_err());
}

#[test]
fn snapshot_lists_groups_and_per_channel_enabled() {
    let _sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_note")).expect("save");
    let snap = snapshot().expect("snapshot");
    assert_eq!(
        snap["channels"],
        serde_json::json!(["workbench", "cursor_ide", "mobile"])
    );
    assert_eq!(
        snap["workbench_only_tools"],
        serde_json::json!(["delete_note"])
    );
    let groups = snap["groups"].as_array().expect("groups");
    assert_eq!(groups[0]["id"], "notes");
    assert_eq!(groups[1]["id"], "todo");
    let mobile: HashSet<&str> = snap["enabled"]["mobile"]
        .as_array()
        .expect("mobile")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(!mobile.contains("create_note"));
    assert!(!mobile.contains("delete_note"));
    assert!(mobile.contains("list_todo_tasks"));
    let workbench: HashSet<&str> = snap["enabled"]["workbench"]
        .as_array()
        .expect("workbench")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(workbench.contains("delete_note"));
}

#[test]
fn delete_note_never_enables_on_non_workbench_even_if_listed() {
    let _sandbox = TestSandbox::new();
    let mut with_delete: Vec<String> = catalog().into_iter().collect();
    with_delete.sort();
    set_enabled("cursor_ide", with_delete.clone()).expect("save full list");
    assert!(!is_enabled("cursor_ide", "delete_note"));
    set_enabled("mobile", with_delete).expect("save full list");
    assert!(!is_enabled("mobile", "delete_note"));
    assert!(is_enabled("workbench", "delete_note"));
}

#[test]
fn listen_filters_list_and_call_through_this_service() {
    let listen = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mcp_protocol_adapter/listen.rs"
    ));
    assert!(
        listen.contains("mcp_channel_tools::enabled_names"),
        "list_tools must filter by channel checkboxes"
    );
    assert!(
        listen.contains("mcp_channel_tools::is_enabled"),
        "call_tool must reject unchecked tools"
    );
    assert!(
        listen.contains("channel: \"mobile\""),
        "/mcp/mobile must use channel mobile while scene_slot stays workbench"
    );
}
