use super::*;
use serde_json::json;

#[test]
fn gh_delete_assets_rejects_missing_url() {
    let r = gh_delete_assets(&json!({}));
    assert_eq!(r.get("_status"), Some(&json!(400)));
    assert!(r
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .contains("url required"));
}

#[test]
fn gh_delete_assets_rejects_invalid_url() {
    let r = gh_delete_assets(&json!({
        "url": "https://example.com/o/r/blob/main/f.md"
    }));
    assert_eq!(r.get("_status"), Some(&json!(400)));
    assert!(r
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .contains("源 URL 格式无效"));
}

#[test]
fn gh_delete_assets_blob_path_without_token() {
    crate::config::secrets::test_secrets_clear();
    let r = gh_delete_assets(&json!({
        "url": "https://github.com/o/r/blob/main/docs/f.md"
    }));
    let err = r.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!err.contains("源 URL 格式无效"));
    assert!(err.contains("获取") || err.contains("GitHub Token"));
}

#[test]
fn collect_tree_blob_paths_skips_gitkeep_and_sorts_deep_first() {
    let tree = json!({
        "tree": [
            { "type": "blob", "path": "docs/a/file.md" },
            { "type": "blob", "path": "docs/a/.gitkeep" },
            { "type": "blob", "path": "docs/a/sub/deep.md" },
            { "type": "tree", "path": "docs/a/sub" }
        ]
    });
    let files = collect_tree_blob_paths(&tree, "docs/a");
    assert_eq!(files.len(), 2);
    assert!(!files.iter().any(|p| p.ends_with(".gitkeep")));
    assert_eq!(files[0], "docs/a/sub/deep.md");
    assert_eq!(files[1], "docs/a/file.md");
}

#[test]
fn collect_tree_blob_paths_empty_when_no_blobs() {
    let tree = json!({
        "tree": [
            { "type": "blob", "path": "other/dir/f.md" },
            { "type": "tree", "path": "docs/a" }
        ]
    });
    let files = collect_tree_blob_paths(&tree, "docs/a");
    assert!(files.is_empty());
}
