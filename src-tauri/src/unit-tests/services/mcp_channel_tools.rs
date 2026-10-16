use std::collections::HashSet;
use std::fs;

use crate::config::paths;
use crate::services::settings::mcp_channel_tools::{
    enabled_names, is_enabled, set_enabled, snapshot, MCP_CHANNELS,
};
use crate::mcp_host::catalog_groups;
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
fn set_enabled_subset_filters_only_that_channel() {
    let sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_doc")).expect("save mobile");
    assert!(!is_enabled("mobile", "create_doc"));
    assert!(is_enabled("mobile", "list_projects"));
    assert!(is_enabled("spark", "create_doc"));
    assert!(is_enabled("cursor_ide", "create_doc"));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let stored: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).expect("read"))
        .expect("json");
    assert!(stored["channels"].get("spark").is_none());
    let mobile_docs = stored["channels"]["mobile"]["docs"]
        .as_array()
        .expect("mobile docs");
    assert!(mobile_docs
        .iter()
        .all(|n| n.as_str() != Some("create_doc")));
    assert!(stored["channels"]["mobile"].get("todo").is_none());
}

#[test]
fn set_enabled_all_tools_omits_channel_key() {
    let _sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_doc")).expect("subset");
    // Mobile cannot enable spark-only tools; "all on" = catalog minus those.
    let all_mobile: Vec<String> = {
        let mut names: Vec<String> = catalog()
            .into_iter()
            .filter(|n| n != "delete_doc")
            .collect();
        names.sort();
        names
    };
    set_enabled("mobile", all_mobile).expect("restore all");
    assert!(is_enabled("mobile", "create_doc"));
    assert!(!is_enabled("mobile", "delete_doc"));
    let path = paths::mcp_channel_tools_path().expect("path");
    assert!(
        !path.exists(),
        "all-on should omit every key and drop the file"
    );
}

#[test]
fn set_enabled_rejects_unknown_channel_and_tool() {
    let _sandbox = TestSandbox::new();
    assert!(set_enabled("not-a-channel", vec!["create_doc".into()]).is_err());
    assert!(set_enabled("mobile", vec!["not_a_tool".into()]).is_err());
}

#[test]
fn snapshot_lists_groups_and_per_channel_enabled() {
    let _sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_doc")).expect("save");
    let snap = snapshot().expect("snapshot");
    assert_eq!(
        snap["channels"],
        serde_json::json!(["spark", "cursor", "codex", "claude", "mobile"])
    );
    assert_eq!(
        snap["spark_only_tools"],
        serde_json::json!(["delete_doc"])
    );
    let groups = snap["groups"].as_array().expect("groups");
    let ids: Vec<_> = groups.iter().filter_map(|g| g["id"].as_str()).collect();
    assert!(ids.contains(&"docs"));
    assert!(ids.contains(&"global"));
    assert!(!ids.contains(&"notes"));
    assert!(!ids.contains(&"knowledge"));
    assert!(!ids.contains(&"todo"));
    let mobile: HashSet<&str> = snap["enabled"]["mobile"]
        .as_array()
        .expect("mobile")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(!mobile.contains("create_doc"));
    assert!(!mobile.contains("delete_doc"));
    assert!(!mobile.contains("list_todo_tasks"));
    let spark: HashSet<&str> = snap["enabled"]["spark"]
        .as_array()
        .expect("spark")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(spark.contains("delete_doc"));
}

#[test]
fn delete_note_never_enables_on_non_spark_even_if_listed() {
    let _sandbox = TestSandbox::new();
    let mut with_delete: Vec<String> = catalog().into_iter().collect();
    with_delete.sort();
    set_enabled("cursor_ide", with_delete.clone()).expect("save full list");
    assert!(!is_enabled("cursor_ide", "delete_doc"));
    set_enabled("mobile", with_delete).expect("save full list");
    assert!(!is_enabled("mobile", "delete_doc"));
    assert!(is_enabled("spark", "delete_doc"));
}

#[test]
fn load_rewrites_retired_search_tools_to_search() {
    let sandbox = TestSandbox::new();
    write_channels(serde_json::json!({
        "cursor_ide": {
            "notes": ["list_notes_categories", "search_notes"],
            "todo": [],
            "knowledge": ["search_knowledge"],
            "global": ["search_document"]
        }
    }));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let grouped = crate::services::settings::mcp_channel_tools::enabled_by_group("cursor_ide");
    assert!(grouped.global.contains(&"search".into()));
    assert!(grouped.docs.contains(&"list_projects".into()));
    assert!(is_enabled("cursor_ide", "search"));
    for retired in ["search_document", "search_notes", "search_knowledge", "list_notes_categories"] {
        assert!(!is_enabled("cursor_ide", retired), "{retired}");
    }
}

fn write_channels(channels: serde_json::Value) {
    let path = paths::mcp_channel_tools_path().expect("path");
    fs::write(&path, serde_json::json!({ "channels": channels }).to_string()).expect("write");
}

#[test]
fn load_folds_notes_and_knowledge_groups_into_docs() {
    let sandbox = TestSandbox::new();
    write_channels(serde_json::json!({
        "cursor_ide": {
            "notes": ["create_note", "update_note", "get_note_digest", "create_notes_category"],
            "knowledge": ["list_knowledge_categories", "get_knowledge_file"],
            "global": ["get_read_later"]
        }
    }));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let grouped = crate::services::settings::mcp_channel_tools::enabled_by_group("cursor_ide");
    assert_eq!(
        grouped.docs,
        vec!["create_doc", "create_project", "get_doc_digest", "get_doc_file", "list_projects", "update_doc"]
    );
    assert!(is_enabled("cursor_ide", "create_doc"));
    assert!(!is_enabled("cursor_ide", "create_note"));
    // Tools that did not exist before stay off in a saved selection.
    assert!(!is_enabled("cursor_ide", "list_workspaces"));
}

#[test]
fn load_rewrites_retired_note_content_name() {
    let sandbox = TestSandbox::new();
    write_channels(serde_json::json!({
        "cursor_ide": {
            "notes": ["list_notes_categories", "get_note_content_by_id"],
            "todo": [],
            "knowledge": []
        },
        "mobile": { "notes": ["get_note_content_by_id"], "knowledge": [] }
    }));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let grouped = crate::services::settings::mcp_channel_tools::enabled_by_group("cursor_ide");
    assert!(!is_enabled("cursor_ide", "get_note_content_by_id"));
    // Desktop: retired name → get_doc_content → get_doc_file.
    assert!(grouped.docs.contains(&"get_doc_file".into()));
    assert!(!is_enabled("cursor_ide", "get_doc_content"));
    // Mobile keeps get_doc_content.
    assert!(is_enabled("mobile", "get_doc_content"));
    assert!(!is_enabled("mobile", "get_doc_file"));
}

#[test]
fn load_rewrites_retired_note_digest_name() {
    let sandbox = TestSandbox::new();
    write_channels(serde_json::json!({
        "cursor_ide": {
            "notes": ["list_notes_categories", "get_note_digest_by_id"],
            "todo": [],
            "knowledge": []
        },
        "mobile": { "notes": ["get_note_digest_by_id"], "knowledge": [] }
    }));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let desktop = crate::services::settings::mcp_channel_tools::enabled_by_group("cursor_ide");
    assert!(desktop.docs.contains(&"get_doc_digest".into()));
    assert!(is_enabled("cursor_ide", "get_doc_digest"));
    assert!(!is_enabled("cursor_ide", "get_note_digest_by_id"));
    let mobile = crate::services::settings::mcp_channel_tools::enabled_by_group("mobile");
    assert!(mobile.docs.contains(&"get_doc_digest".into()));
    assert!(is_enabled("mobile", "get_doc_digest"));
}

#[test]
fn set_enabled_accepts_a_retired_name_as_its_replacement() {
    let _sandbox = TestSandbox::new();
    set_enabled("cursor_ide", vec!["create_note".into(), "search_notes".into()]).expect("save");
    assert!(is_enabled("cursor_ide", "create_doc"));
    assert!(is_enabled("cursor_ide", "search"));
    assert!(!is_enabled("cursor_ide", "update_doc"));
}

#[test]
fn load_migrates_legacy_flat_channel_lists() {
    let sandbox = TestSandbox::new();
    let path = paths::mcp_channel_tools_path().expect("path");
    fs::write(
        &path,
        serde_json::json!({
            "channels": {
                "mobile": ["list_notes_categories", "list_todo_tasks"]
            }
        })
        .to_string(),
    )
    .expect("write legacy");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    let grouped = crate::services::settings::mcp_channel_tools::enabled_by_group("mobile");
    assert!(grouped.docs.contains(&"list_projects".into()));
    assert!(is_enabled("mobile", "list_projects"));
    assert!(!is_enabled("mobile", "list_todo_tasks"));
}

#[test]
fn load_ignores_retired_get_all_notes_catalog() {
    let sandbox = TestSandbox::new();
    write_channels(serde_json::json!({
        "cursor_ide": {
            "notes": ["get_all_notes_catalog", "create_note"],
            "knowledge": []
        },
        "mobile": ["get_all_notes_catalog", "get_note_digest"]
    }));
    let path = paths::mcp_channel_tools_path().expect("path");
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    for channel in ["cursor_ide", "mobile"] {
        let grouped = crate::services::settings::mcp_channel_tools::enabled_by_group(channel);
        assert!(
            !grouped.docs.contains(&"get_all_notes_catalog".into()),
            "{channel}: retired name must be dropped on load"
        );
        assert!(!is_enabled(channel, "get_all_notes_catalog"), "{channel}");
    }
    assert!(is_enabled("cursor_ide", "create_doc"));
    assert!(is_enabled("mobile", "get_doc_digest"));
    let snap = snapshot().expect("snapshot must not fail on a stored retired name");
    let listed: HashSet<&str> = snap["groups"]
        .as_array()
        .expect("groups")
        .iter()
        .flat_map(|g| g["tools"].as_array().expect("tools"))
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(!listed.contains("get_all_notes_catalog"));
    assert!(listed.contains("list_projects"));
}

#[test]
fn snapshot_exposes_grouped_enabled() {
    let _sandbox = TestSandbox::new();
    set_enabled("mobile", without("create_doc")).expect("save");
    let snap = snapshot().expect("snapshot");
    assert!(snap["enabled_grouped"]["mobile"]["docs"]
        .as_array()
        .expect("mobile notes grouped")
        .iter()
        .all(|n| n.as_str() != Some("create_doc")));
}

#[test]
fn listen_filters_list_and_call_through_settings_catalog() {
    let listen = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp_host/server/listen.rs"
    ));
    assert!(
        listen.contains("mcp_catalog::enabled_grouped"),
        "list_tools must filter by grouped channel checkboxes"
    );
    assert!(
        listen.contains("mcp_catalog::is_enabled"),
        "call_tool must reject unchecked tools"
    );
    assert!(
        listen.contains("channel: \"mobile\""),
        "/mcp/mobile must use channel mobile while scene_slot stays spark"
    );
}

#[path = "mcp_channel_availability.rs"]
mod availability;
