use std::fs;

use serde_json::json;

use super::*;
use crate::services::docs::create_workspace;
use crate::test_support::TestSandbox;

const DOC: &str = "abcdefabcdefabcdefabcdefabcdefab";

#[test]
fn a_missing_file_reads_empty_and_is_not_written() {
    let sandbox = TestSandbox::new();
    assert_eq!(get_viewer_state(&json!({}))["id"], "");
    assert!(!sandbox.cache_dir().join("workspace-viewer-state/default.json").exists());
}

#[test]
fn set_then_get_round_trips_and_trims_the_id() {
    let sandbox = TestSandbox::new();
    assert_eq!(set_viewer_state(&json!({ "id": format!("  {DOC}  ") }))["id"], DOC);
    assert_eq!(get_viewer_state(&json!({}))["id"], DOC);
    assert!(sandbox.cache_dir().join("workspace-viewer-state/default.json").is_file());
}

#[test]
fn an_empty_id_forgets_the_document() {
    let _sandbox = TestSandbox::new();
    set_viewer_state(&json!({ "id": DOC }));
    assert_eq!(set_viewer_state(&json!({ "id": "" }))["id"], "");
    assert_eq!(get_viewer_state(&json!({}))["id"], "");
}

#[test]
fn each_workspace_remembers_its_own_document() {
    let _sandbox = TestSandbox::new();
    let other = create_workspace(&json!({ "name": "Memory" }))["workspace"]["id"]
        .as_str()
        .expect("id")
        .to_string();
    set_viewer_state(&json!({ "workspace": "default", "id": DOC }));
    assert_eq!(get_viewer_state(&json!({ "workspace": other }))["id"], "");
    assert_eq!(get_viewer_state(&json!({ "workspace": "default" }))["id"], DOC);
}

#[test]
fn an_unknown_workspace_is_refused() {
    let _sandbox = TestSandbox::new();
    assert_eq!(get_viewer_state(&json!({ "workspace": "nope" }))["_status"], 404);
    assert_eq!(set_viewer_state(&json!({ "workspace": "nope", "id": DOC }))["_status"], 404);
}

#[test]
fn a_corrupt_file_reads_empty_without_overwrite() {
    let sandbox = TestSandbox::new();
    let path = sandbox.cache_dir().join("workspace-viewer-state/default.json");
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, "{not-json").expect("write");
    assert_eq!(get_viewer_state(&json!({}))["id"], "");
    assert_eq!(fs::read_to_string(&path).expect("read"), "{not-json");
}
