use super::*;
use serde_json::json;
use std::fs;

#[test]
fn write_json_round_trip_deep_equal() {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("ann.json");
    let data = json!({
        "done": true,
        "raw": { "comments": [{ "id": "a", "text": "hi" }] }
    });
    write_json(&path, &data).expect("write");
    let read: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    assert_eq!(read, data);
    let pretty = fs::read_to_string(&path).expect("read");
    assert!(pretty.contains('\n'));
    assert!(pretty.contains("done"));
}

#[test]
fn write_json_empty_object_deletes_file() {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("ann.json");
    fs::write(&path, r#"{"done":true}"#).expect("seed");
    write_json(&path, &json!({})).expect("write");
    assert!(!path.exists());
}

#[test]
fn write_json_strips_empty_layer_dicts_then_deletes() {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("ann.json");
    fs::write(&path, r#"{"raw":{}}"#).expect("seed");
    write_json(&path, &json!({ "raw": {}, "digest": {} })).expect("write");
    assert!(!path.exists());
}
