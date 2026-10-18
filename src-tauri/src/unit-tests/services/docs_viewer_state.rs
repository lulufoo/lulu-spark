use std::fs;

use serde_json::json;

use super::*;
use crate::services::docs::create_workspace;
use crate::test_support::TestSandbox;

const DOC: &str = "abcdefabcdefabcdefabcdefabcdefab";

fn state_path(sandbox: &TestSandbox) -> std::path::PathBuf {
    sandbox.cache_dir().join("workspace-viewer-state/default.json")
}

#[test]
fn a_missing_file_reads_empty_and_is_not_written() {
    let sandbox = TestSandbox::new();
    let got = get_viewer_state(&json!({}));
    assert_eq!(got, json!({ "version": 2, "project": "", "projects": {} }));
    assert!(!state_path(&sandbox).exists());
}

#[test]
fn the_directory_view_records_node_and_doc_for_its_project() {
    let sandbox = TestSandbox::new();
    let set = set_viewer_state(&json!({
        "project": "inbox", "node": "inbox/plans", "doc": format!("  {DOC}  ")
    }));
    let want = json!({
        "version": 2,
        "project": "inbox",
        "projects": { "inbox": { "node": "inbox/plans", "doc": DOC, "date": "" } }
    });
    assert_eq!(set, want);
    assert_eq!(get_viewer_state(&json!({})), want);
    assert!(state_path(&sandbox).is_file());
}

#[test]
fn the_timeline_records_a_date_and_all_is_remembered_as_an_empty_project() {
    let _sandbox = TestSandbox::new();
    set_viewer_state(&json!({ "project": "inbox", "date": "20260101" }));
    let got = set_viewer_state(&json!({ "project": "" }));
    assert_eq!(got["project"], "");
    assert_eq!(got["projects"]["inbox"]["date"], "20260101", "All keeps the project's own entry");
    assert_eq!(get_viewer_state(&json!({}))["project"], "");
}

#[test]
fn fields_left_out_are_kept_and_empty_ones_are_cleared() {
    let _sandbox = TestSandbox::new();
    set_viewer_state(&json!({ "project": "a", "node": "a/x", "doc": DOC, "date": "20260101" }));
    let kept = set_viewer_state(&json!({ "project": "a", "doc": "" }));
    assert_eq!(kept["projects"]["a"], json!({ "node": "a/x", "doc": "", "date": "20260101" }));
    let again = set_viewer_state(&json!({ "project": "a", "node": "a/y" }));
    assert_eq!(again["projects"]["a"], json!({ "node": "a/y", "doc": "", "date": "20260101" }));
}

#[test]
fn an_entry_with_every_field_cleared_is_dropped() {
    let _sandbox = TestSandbox::new();
    set_viewer_state(&json!({ "project": "a", "node": "a/x", "doc": DOC }));
    set_viewer_state(&json!({ "project": "b", "date": "20260101" }));
    let got = set_viewer_state(&json!({ "project": "a", "node": "", "doc": "" }));
    assert!(got["projects"].get("a").is_none(), "no record for a");
    assert!(got["projects"].get("b").is_some(), "b is untouched");
}

#[test]
fn projects_keep_separate_entries() {
    let _sandbox = TestSandbox::new();
    set_viewer_state(&json!({ "project": "a", "node": "a/x", "doc": DOC }));
    set_viewer_state(&json!({ "project": "b", "node": "b/y" }));
    let got = get_viewer_state(&json!({}));
    assert_eq!(got["project"], "b");
    assert_eq!(got["projects"]["a"]["node"], "a/x");
    assert_eq!(got["projects"]["b"], json!({ "node": "b/y", "doc": "", "date": "" }));
}

#[test]
fn each_workspace_remembers_its_own_state() {
    let _sandbox = TestSandbox::new();
    let other = create_workspace(&json!({ "workspace_name": "Memory" }))["workspace_id"]
        .as_str()
        .expect("id")
        .to_string();
    set_viewer_state(&json!({ "workspace": "default", "project": "a", "doc": DOC }));
    assert_eq!(get_viewer_state(&json!({ "workspace": other }))["projects"], json!({}));
    assert_eq!(get_viewer_state(&json!({ "workspace": "default" }))["projects"]["a"]["doc"], DOC);
}

#[test]
fn bad_input_is_refused_and_changes_nothing() {
    let sandbox = TestSandbox::new();
    assert_eq!(get_viewer_state(&json!({ "workspace": "nope" }))["_status"], 404);
    assert_eq!(set_viewer_state(&json!({ "workspace": "nope", "project": "a" }))["_status"], 404);
    assert_eq!(set_viewer_state(&json!({ "doc": DOC }))["_status"], 400, "project is required");
    assert_eq!(set_viewer_state(&json!({ "project": 3 }))["_status"], 400);
    assert_eq!(set_viewer_state(&json!({ "project": "a", "doc": 3 }))["_status"], 400);
    assert!(!state_path(&sandbox).exists());
}

#[test]
fn the_old_id_file_and_a_corrupt_file_read_empty_without_overwrite() {
    let sandbox = TestSandbox::new();
    let path = state_path(&sandbox);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    let empty = json!({ "version": 2, "project": "", "projects": {} });
    for text in [format!(r#"{{"version":1,"id":"{DOC}"}}"#), "{not-json".to_string()] {
        fs::write(&path, &text).expect("write");
        assert_eq!(get_viewer_state(&json!({})), empty, "{text}");
        assert_eq!(fs::read_to_string(&path).expect("read"), text, "read leaves the file alone");
    }
}

#[test]
fn a_set_after_the_old_file_starts_a_fresh_state() {
    let sandbox = TestSandbox::new();
    let path = state_path(&sandbox);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, format!(r#"{{"version":1,"id":"{DOC}"}}"#)).expect("write");
    let got = set_viewer_state(&json!({ "project": "a", "date": "20260101" }));
    assert_eq!(got["projects"], json!({ "a": { "node": "", "doc": "", "date": "20260101" } }));
}
