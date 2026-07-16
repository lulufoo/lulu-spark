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

const PLAN_TASK_WRITE_COMMANDS: &[&str] = &[
    "create_plan_task",
    "delete_plan_task",
    "add_plan_sub",
    "delete_plan_sub",
];

const PLAN_TASK_LIFECYCLE_COMMANDS: &[&str] = &[
    "read_plan_md",
    "update_plan_md",
    "complete_plan_sub",
    "abandon_plan_sub",
    "update_plan_sub",
    "update_plan_master_title",
];

const NOTE_WRITE_COMMANDS: &[&str] = &["archive_document"];

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
fn plan_task_write_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = PLAN_TASK_WRITE_COMMANDS
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
fn plan_task_lifecycle_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let missing: Vec<_> = PLAN_TASK_LIFECYCLE_COMMANDS
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
fn plan_task_fm4_and_lifecycle_commands_coexist_in_acl() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    let allow = parse_write_api_toml_allow(&toml);
    let all: Vec<_> = PLAN_TASK_WRITE_COMMANDS
        .iter()
        .chain(PLAN_TASK_LIFECYCLE_COMMANDS.iter())
        .copied()
        .collect();
    let missing: Vec<_> = all.iter().filter(|cmd| !allow.contains(**cmd)).copied().collect();
    assert!(
        missing.is_empty(),
        "plan task commands missing from write-api.toml ACL: {missing:?}"
    );
    assert_eq!(
        all.len(),
        PLAN_TASK_WRITE_COMMANDS.len() + PLAN_TASK_LIFECYCLE_COMMANDS.len(),
        "expected nine distinct plan task ACL entries"
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
