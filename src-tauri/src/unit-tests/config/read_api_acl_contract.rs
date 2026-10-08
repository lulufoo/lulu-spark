//! Tests: `permissions/read-api.toml` ↔ `gen/schemas/acl-manifests.json` ↔ frontend invoke map.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parse_read_api_toml_allow(text: &str) -> BTreeSet<String> {
    let block = text
        .split("identifier = \"read-api\"")
        .nth(1)
        .and_then(|s| s.split("commands.allow = [").nth(1))
        .and_then(|s| s.split(']').next())
        .expect("read-api commands.allow block");
    block
        .split('"')
        .enumerate()
        .filter_map(|(i, s)| if i % 2 == 1 { Some(s.to_string()) } else { None })
        .collect()
}

fn parse_acl_manifest_allow(text: &str) -> BTreeSet<String> {
    let v: serde_json::Value = serde_json::from_str(text).expect("acl json");
    let arr = v["__app-acl__"]["permissions"]["read-api"]["commands"]["allow"]
        .as_array()
        .expect("read-api allow array");
    arr.iter()
        .map(|x| x.as_str().expect("cmd string").to_string())
        .collect()
}

/// 与 `frontend/src/host/readApiInvokeMap.ts` 中 `cmd` 字段保持同步。
const INVOKE_MAP_COMMANDS: &[&str] = &[
    "get_notes_index",
    "get_notes_file",
    "read_abs_file",
    "get_notes_asset",
    "list_notes_categories",
    "get_topics",
    "search_knowledge",
    "search_spark",
    "get_annotations",
    "get_annotation",
    "get_draft",
    "get_note_draft",
    "get_config",
    "kb_read",
    "get_kb_asset",
    "kb_list",
    "kb_annotation",
    "get_kb_hide_patterns",
    "get_kb_viewer_state",
    "fetch_link_title",
    "get_tags_registry",
    "get_read_later",
    "get_todo_tasks",
    "get_doc_highlights",
];

const SEDIMENT_KB_READ_COMMANDS: &[&str] = &[
    "get_sediment_kb_categories",
    "get_sediment_kb_repos",
];

const BIND_READ_COMMANDS: &[&str] = &["read_bind_session"];

const MCP_CHANNEL_TOOLS_READ_COMMANDS: &[&str] = &["get_mcp_channel_tools"];

const MCP_TICKET_VIEW_READ_COMMANDS: &[&str] = &["get_mcp_ticket_view"];

const MESSAGE_CENTER_READ_COMMANDS: &[&str] = &["get_message_channel_unread"];

const PACKAGE_SNAPSHOT_READ_COMMANDS: &[&str] = &["get_package_snapshot"];

const DEFAULT_CAPABILITY_PERMISSIONS: &[&str] = &[
    "core:default",
    "core:window:allow-start-dragging",
    "read-api",
    "write-api",
    "sync-api",
    "search-api",
    "opener:default",
    "dialog:default",
];

#[test]
fn read_api_toml_and_acl_manifest_allow_lists_match() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let acl = fs::read_to_string(root.join("gen/schemas/acl-manifests.json")).expect("acl");
    let toml_allow = parse_read_api_toml_allow(&toml);
    let acl_allow = parse_acl_manifest_allow(&acl);
    assert_eq!(toml_allow, acl_allow, "read-api.toml vs acl-manifests.json");
}

#[test]
fn invoke_map_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = INVOKE_MAP_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn mcp_ticket_view_read_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = MCP_TICKET_VIEW_READ_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn mcp_channel_tools_read_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = MCP_CHANNEL_TOOLS_READ_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn bind_read_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = BIND_READ_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn sediment_kb_read_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = SEDIMENT_KB_READ_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn kb_doc_count_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    assert!(
        allow.contains("kb_doc_count"),
        "kb_doc_count must be in read-api.toml ACL"
    );
}

#[test]
fn settings_github_infer_commands_are_not_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    for cmd in ["infer_github_user_url", "check_spark_root", "get_status"] {
        assert!(!allow.contains(cmd), "{cmd} must not stay in read-api.toml");
    }
}

#[test]
fn package_snapshot_read_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = PACKAGE_SNAPSHOT_READ_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn message_center_read_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    let missing: Vec<_> = MESSAGE_CENTER_READ_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from read-api.toml ACL: {missing:?}"
    );
}

#[test]
fn default_capability_grants_existing_read_and_write_api_only() {
    let root = manifest_dir();
    let raw = fs::read_to_string(root.join("capabilities/default.json")).expect("default.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("default.json");
    let perms: Vec<_> = v["permissions"]
        .as_array()
        .expect("permissions array")
        .iter()
        .map(|p| p.as_str().expect("permission string").to_string())
        .collect();
    assert_eq!(
        perms,
        DEFAULT_CAPABILITY_PERMISSIONS,
        "capabilities/default.json must keep granting existing read-api / write-api and not open another capability"
    );
}
