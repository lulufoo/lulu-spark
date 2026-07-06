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

/// 与 `frontend/js/readApiInvokeMap.js` 中 `cmd` 字段保持同步。
const INVOKE_MAP_COMMANDS: &[&str] = &[
    "get_corpus_index",
    "get_corpus_file",
    "get_corpus_asset",
    "get_topics",
    "search_knowledge",
    "search_workbench",
    "get_annotations",
    "get_annotation",
    "get_draft",
    "get_config",
    "infer_github_user_url",
    "check_workbench_knowledge_root",
    "get_status",
    "kb_read",
    "kb_list",
    "kb_annotation",
    "kb_status",
    "get_repo_dirs",
    "check_file",
    "fetch_link_title",
    "get_tags_registry",
    "get_read_later",
];

const SEDIMENT_KB_READ_COMMANDS: &[&str] = &[
    "get_sediment_kb_categories",
    "get_sediment_kb_repos",
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
fn settings_github_infer_commands_are_acl_allowed() {
    let root = manifest_dir();
    let toml = fs::read_to_string(root.join("permissions/read-api.toml")).expect("toml");
    let allow = parse_read_api_toml_allow(&toml);
    for cmd in ["infer_github_user_url", "check_workbench_knowledge_root"] {
        assert!(allow.contains(cmd), "{cmd} must be in read-api.toml");
    }
}
