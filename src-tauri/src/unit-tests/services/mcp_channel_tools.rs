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
        assert_eq!(enabled_names(channel), all, "{channel}");
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
    let all: Vec<String> = {
        let mut names: Vec<String> = catalog().into_iter().collect();
        names.sort();
        names
    };
    set_enabled("mobile", all).expect("restore all");
    assert!(is_enabled("mobile", "create_note"));
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
    assert!(mobile.contains("list_todo_tasks"));
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
