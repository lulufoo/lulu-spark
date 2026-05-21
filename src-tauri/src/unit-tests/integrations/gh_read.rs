use super::*;

#[test]
fn parse_concatenated_arrays_merges() {
    let raw = r#"[{"full_name":"a/b"}][{"full_name":"c/d"}]"#;
    let items = parse_concatenated_json_arrays(raw);
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_repo_dirs_filters_dot_dirs() {
    let items = serde_json::json!([
      {"name": "docs", "type": "dir"},
      {"name": ".hidden", "type": "dir"},
      {"name": "readme.md", "type": "file"}
    ]);
    let v = parse_repo_dirs(&items).expect("parse");
    assert_eq!(v["dirs"], json!(["docs"]));
}

#[test]
fn check_file_rejects_unknown_repo() {
    let dir = tempfile::tempdir().expect("tmp");
    let v = check_file_json(dir.path(), "unknown/foo", "a.md");
    assert!(v.get("error").is_some());
}

#[test]
fn type_meta_fetch_limit_is_reasonable() {
    assert!(TYPE_META_FETCH_LIMIT > 0);
    assert!(TYPE_META_FETCH_LIMIT <= 100);
}
