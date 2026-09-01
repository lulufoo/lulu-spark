//! Tests: `permissions/write-api.toml` ↔ `gen/schemas/acl-manifests.json`.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parse_write_api_toml_allow(text: &str) -> BTreeSet<String> {
    let block = text
        .split("identifier = \"write-api\"")
        .nth(1)
        .and_then(|s| s.split("commands.allow = [").nth(1))
        .and_then(|s| s.split(']').next())
        .expect("write-api commands.allow block");
    block
        .split('"')
        .enumerate()
        .filter_map(|(i, s)| if i % 2 == 1 { Some(s.to_string()) } else { None })
        .collect()
}

fn parse_acl_manifest_allow(text: &str) -> BTreeSet<String> {
    let v: serde_json::Value = serde_json::from_str(text).expect("acl json");
    let arr = v["__app-acl__"]["permissions"]["write-api"]["commands"]["allow"]
        .as_array()
        .expect("write-api allow array");
    arr.iter()
        .map(|x| x.as_str().expect("cmd string").to_string())
        .collect()
}

const SEDIMENT_KB_WRITE_COMMANDS: &[&str] = &[
    "sediment_kb_add_repo",
    "sediment_kb_remove_repo",
    "sediment_kb_update_repo_category",
    "sediment_kb_add_category",
    "sediment_kb_rename_category",
    "sediment_kb_remove_category",
];

const TODO_TASK_WRITE_COMMANDS: &[&str] = &[
    "create_todo_task",
    "delete_todo_task",
    "add_todo_sub",
    "delete_todo_sub",
];

const TODO_TASK_LIFECYCLE_COMMANDS: &[&str] = &[
    "read_todo_md",
    "update_todo_md",
    "complete_todo",
    "abandon_todo_sub",
    "update_todo_sub",
    "update_todo_master_title",
    "set_todo_master_status",
];

const TODO_TASK_ATTACHMENT_COMMANDS: &[&str] = &[
    "stage_todo_attachment_source",
    "add_todo_attachment",
    "list_todo_attachments",
    "read_todo_attachment",
    "save_todo_attachment",
    "delete_todo_attachment",
];

const TODO_TASK_COMMENT_COMMANDS: &[&str] = &[
    "list_todo_comments",
    "add_todo_comment",
    "update_todo_comment",
    "delete_todo_comment",
];

const TODO_TASK_CATEGORY_COMMANDS: &[&str] = &[
    "list_todo_categories",
    "create_todo_category",
    "delete_todo_category",
    "set_todo_category",
];

const NOTE_WRITE_COMMANDS: &[&str] = &[
    "create_note",
    "create_notes_category",
    "update_notes_category",
    "delete_notes_category",
];

const DOC_HIGHLIGHTS_WRITE_COMMANDS: &[&str] = &["update_doc_highlights"];

const MCP_OAUTH_WRITE_COMMANDS: &[&str] = &[
    "issue_cursor_ide_ticket",
    "rotate_cursor_ide_ticket",
    "revoke_mcp_slot_ticket",
    "revoke_mcp_device_ticket",
];

const MCP_CHANNEL_TOOLS_WRITE_COMMANDS: &[&str] = &["set_mcp_channel_tools"];

const BIND_WRITE_COMMANDS: &[&str] = &["issue_bind"];

const AI_ASSISTANT_WRITE_COMMANDS: &[&str] = &[
    "open_ai_assistant",
    "present_ai_assistant",
    "ensure_ai_assistant_session",
    "get_ai_assistant_binding",
    "select_chat_session",
    "create_chat_session",
    "delete_chat_session",
    "agent_chat_turn",
];

#[test]
fn write_api_toml_and_acl_manifest_allow_lists_match() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let acl = fs::read_to_string(root.join("gen/schemas/acl-manifests.json")).expect("acl");
    let toml_allow = parse_write_api_toml_allow(&toml);
    let acl_allow = parse_acl_manifest_allow(&acl);
    assert_eq!(toml_allow, acl_allow, "write-api.toml vs acl-manifests.json");
}

#[test]
fn sediment_kb_write_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = SEDIMENT_KB_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn todo_task_write_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = TODO_TASK_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn todo_task_lifecycle_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = TODO_TASK_LIFECYCLE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
    assert!(
        !allow.contains("complete_plan_sub"),
        "legacy complete_plan_sub must not remain in write-api ACL"
    );
}

#[test]
fn todo_task_fm4_and_lifecycle_commands_coexist_in_acl() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let all: Vec<_> = TODO_TASK_WRITE_COMMANDS
        .iter()
        .chain(TODO_TASK_LIFECYCLE_COMMANDS.iter())
        .chain(TODO_TASK_ATTACHMENT_COMMANDS.iter())
        .chain(TODO_TASK_COMMENT_COMMANDS.iter())
        .chain(TODO_TASK_CATEGORY_COMMANDS.iter())
        .copied()
        .collect();
    let missing: Vec<_> = all.iter().filter(|cmd| !allow.contains(**cmd)).copied().collect();
    assert!(
        missing.is_empty(),
        "plan task commands missing from write-api.toml ACL: {missing:?}"
    );
    assert_eq!(
        all.len(),
        TODO_TASK_WRITE_COMMANDS.len()
            + TODO_TASK_LIFECYCLE_COMMANDS.len()
            + TODO_TASK_ATTACHMENT_COMMANDS.len()
            + TODO_TASK_COMMENT_COMMANDS.len()
            + TODO_TASK_CATEGORY_COMMANDS.len(),
        "expected twenty-three distinct plan task ACL entries"
    );
}

#[test]
fn todo_task_category_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = TODO_TASK_CATEGORY_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn todo_task_attachment_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = TODO_TASK_ATTACHMENT_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn todo_task_comment_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = TODO_TASK_COMMENT_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn bind_write_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = BIND_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn mcp_channel_tools_write_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = MCP_CHANNEL_TOOLS_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn mcp_oauth_write_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = MCP_OAUTH_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn doc_highlights_write_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = DOC_HIGHLIGHTS_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn note_archive_write_command_is_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = NOTE_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}

#[test]
fn ai_assistant_write_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = AI_ASSISTANT_WRITE_COMMANDS
        .iter()
        .filter(|cmd| !allow.contains(**cmd))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "commands missing from write-api.toml ACL: {missing:?}"
    );
}
