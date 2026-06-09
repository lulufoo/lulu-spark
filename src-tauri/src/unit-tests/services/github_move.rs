use super::*;
use serde_json::json;

#[test]
fn gh_move_assets_rejects_missing_urls() {
    let r = gh_move_assets(&json!({}));
    assert_eq!(r.get("_status"), Some(&json!(400)));
    assert!(r.get("error").and_then(|v| v.as_str()).unwrap_or("").contains("required"));
}

#[test]
fn gh_move_assets_rejects_invalid_dst_url() {
    let r = gh_move_assets(&json!({
        "src_url": "https://github.com/o/r/blob/main/f.md",
        "dst_dir_url": "not-a-github-url"
    }));
    assert_eq!(r.get("_status"), Some(&json!(400)));
    assert!(r.get("error").and_then(|v| v.as_str()).unwrap_or("").contains("目标目录"));
}

#[test]
fn gh_move_assets_rejects_invalid_src_url() {
    let r = gh_move_assets(&json!({
        "src_url": "https://example.com/o/r/blob/main/f.md",
        "dst_dir_url": "https://github.com/o/r/tree/main/docs"
    }));
    assert_eq!(r.get("_status"), Some(&json!(400)));
    assert!(r.get("error").and_then(|v| v.as_str()).unwrap_or("").contains("源 URL"));
}
