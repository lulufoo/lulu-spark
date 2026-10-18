use std::fs;

use serde_json::{json, Value};

use super::*;
use crate::services::docs::{create_doc_content, create_workspace};
use crate::services::search_index::collect_workspace_docs;
use crate::services::spark_read::get_docs_index;
use crate::test_support::{seed_note_entries, TestSandbox};

fn patterns(value: &Value) -> Vec<String> {
    value["patterns"]
        .as_array()
        .unwrap_or_else(|| panic!("expected patterns, got {value}"))
        .iter()
        .map(|row| row["pattern"].as_str().expect("pattern").to_string())
        .collect()
}

fn id_of(value: &Value, pattern: &str) -> String {
    value["patterns"]
        .as_array()
        .expect("patterns")
        .iter()
        .find(|row| row["pattern"] == pattern)
        .and_then(|row| row["id"].as_str())
        .unwrap_or_else(|| panic!("no row for {pattern} in {value}"))
        .to_string()
}

fn status(value: &Value) -> u64 {
    value["_status"].as_u64().unwrap_or_else(|| panic!("expected an error, got {value}"))
}

fn new_workspace(name: &str) -> String {
    let made = create_workspace(&json!({ "workspace_name": name }));
    made["workspace_id"].as_str().expect("workspace id").to_string()
}

fn in_workspace(workspace: &str) -> Value {
    json!({ "workspace_id": workspace })
}

#[test]
fn first_read_seeds_the_four_default_rules_and_writes_the_workspace_file() {
    let sandbox = TestSandbox::new();
    let listed = list_hide_patterns(&json!({}));
    assert_eq!(patterns(&listed), SEED_PATTERNS);
    let ids: Vec<_> = listed["patterns"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["seed00", "seed01", "seed02", "seed03"]);
    assert!(sandbox.cache_dir().join("workspace-hide-patterns/default.json").is_file());
}

#[test]
fn first_read_takes_the_rules_of_the_old_global_list_when_it_exists() {
    let sandbox = TestSandbox::new();
    let legacy = sandbox.cache_dir().join("knowledge-hide-patterns.json");
    fs::create_dir_all(legacy.parent().unwrap()).expect("mkdir");
    fs::write(
        &legacy,
        r#"{"version":1,"patterns":[{"id":"seed00","pattern":"^\\.git$"},{"id":"mine","pattern":"^\\.cursor$"}]}"#,
    )
    .expect("write legacy");

    let listed = list_hide_patterns(&json!({}));
    assert_eq!(patterns(&listed), vec![r"^\.git$".to_string(), r"^\.cursor$".to_string()]);
    assert!(legacy.is_file(), "the old list is only read, never removed");
}

#[test]
fn rules_belong_to_one_workspace() {
    let _sandbox = TestSandbox::new();
    let other = new_workspace("Memory");
    let added = add_hide_pattern(&json!({ "workspace_id": "default", "pattern": "^draft$" }));
    assert!(patterns(&added).contains(&"^draft$".to_string()));

    assert!(!patterns(&list_hide_patterns(&in_workspace(&other))).contains(&"^draft$".to_string()));
    assert!(patterns(&list_hide_patterns(&in_workspace("default"))).contains(&"^draft$".to_string()));
}

#[test]
fn an_unknown_workspace_is_refused() {
    let _sandbox = TestSandbox::new();
    assert_eq!(status(&list_hide_patterns(&in_workspace("nope"))), 404);
    assert_eq!(status(&add_hide_pattern(&json!({ "workspace_id": "nope", "pattern": "^x$" }))), 404);
}

#[test]
fn add_rejects_empty_invalid_and_duplicate_rules() {
    let _sandbox = TestSandbox::new();
    assert_eq!(status(&add_hide_pattern(&json!({ "pattern": "" }))), 400);
    assert_eq!(status(&add_hide_pattern(&json!({ "pattern": "[" }))), 400);
    assert!(add_hide_pattern(&json!({ "pattern": "^draft$" })).get("error").is_none());
    let dup = add_hide_pattern(&json!({ "pattern": "^draft$" }));
    assert_eq!((status(&dup), dup["error"].as_str()), (409, Some("duplicate pattern")));
}

#[test]
fn update_and_remove_work_by_id() {
    let _sandbox = TestSandbox::new();
    let added = add_hide_pattern(&json!({ "pattern": r"^\.xxx$" }));
    let id = id_of(&added, r"^\.xxx$");

    let updated = update_hide_pattern(&json!({ "id": id, "pattern": r"^\.yyy$" }));
    assert!(patterns(&updated).contains(&r"^\.yyy$".to_string()));
    assert!(!patterns(&updated).contains(&r"^\.xxx$".to_string()));
    assert_eq!(status(&update_hide_pattern(&json!({ "id": "ghost", "pattern": "^a$" }))), 404);
    assert_eq!(status(&update_hide_pattern(&json!({ "id": id, "pattern": "[" }))), 400);

    let removed = remove_hide_pattern(&json!({ "id": id }));
    assert!(!patterns(&removed).contains(&r"^\.yyy$".to_string()));
    assert_eq!(status(&remove_hide_pattern(&json!({ "id": id }))), 404);
}

#[test]
fn a_corrupt_file_reads_as_no_rules_and_is_not_overwritten() {
    let sandbox = TestSandbox::new();
    let path = sandbox.cache_dir().join("workspace-hide-patterns/default.json");
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, "{not-json").expect("write");

    assert!(patterns(&list_hide_patterns(&json!({}))).is_empty());
    assert!(compiled_hide_regexes("default").is_empty());
    assert_eq!(fs::read_to_string(&path).expect("read"), "{not-json");
}

#[test]
fn a_path_is_hidden_when_any_segment_matches_any_rule() {
    let rules = [Regex::new(r"^\.git$").unwrap(), Regex::new(r"^\.cache$").unwrap()];
    assert!(path_is_hidden(".git/config.md", &rules));
    assert!(path_is_hidden("topic/.cache/stray.md", &rules));
    assert!(path_is_hidden("a/b/.git", &rules));
    assert!(!path_is_hidden("topic/git.md", &rules));
    assert!(!path_is_hidden("topic/a.md", &rules));
    assert!(!path_is_hidden(".git/config.md", &[]));
}

#[test]
fn the_index_lists_no_document_under_a_hidden_segment() {
    let _sandbox = TestSandbox::new();
    seed_note_entries(&json!({
        "n1": { "common_path": "ai/topic/keep.md", "layers": ["raw"] },
        "n2": { "common_path": "ai/.git/stray.md", "layers": ["raw"] },
        "n3": { "common_path": "ai/.cursor/rules.md", "layers": ["raw"] },
    }));
    // Default rules hide `.git` only; `.cursor` is hidden once a rule says so.
    let first = get_docs_index(None);
    let mut shown: Vec<_> = first["entries"].as_object().unwrap().keys().cloned().collect();
    shown.sort();
    assert_eq!(shown, vec!["n1".to_string(), "n3".to_string()]);

    add_hide_pattern(&json!({ "pattern": r"^\.cursor$" }));
    let second = get_docs_index(None);
    let shown: Vec<_> = second["entries"].as_object().unwrap().keys().cloned().collect();
    assert_eq!(shown, vec!["n1".to_string()]);

    let other = new_workspace("Memory");
    assert_eq!(
        get_docs_index(Some(&other))["entries"].as_object().unwrap().len(),
        0,
        "another workspace's index is its own"
    );
}

#[test]
fn search_collect_skips_a_hidden_document() {
    let _sandbox = TestSandbox::new();
    let created = create_doc_content(&json!({ "title": "T", "content": "# 标题\n\n这是足够长的正文内容。\n", "digest": "never", "source_type": "summary" }));
    assert!(created["id"].as_str().is_some(), "create failed: {created}");
    assert_eq!(collect_workspace_docs().expect("collect").len(), 1);

    // The document lives under `notes/` inside its project.
    let hidden = add_hide_pattern(&json!({ "pattern": "^notes$" }));
    assert!(hidden.get("error").is_none(), "{hidden}");
    assert!(collect_workspace_docs().expect("collect").is_empty());
}
