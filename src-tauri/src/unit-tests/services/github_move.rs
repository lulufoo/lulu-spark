use super::*;

#[test]
fn parse_blob_url() {
    let u = "https://github.com/o/r/blob/main/path/to/f.md";
    let p = parse_github_blob(u).expect("blob");
    assert_eq!(p.0, "o");
    assert_eq!(p.3, "path/to/f.md");
}

#[test]
fn parse_dst_url_tree_directory() {
    let u = "https://github.com/o/r/tree/main/docs";
    let p = parse_github_dst(u).expect("dst");
    assert_eq!(p.0, "o");
    assert_eq!(p.1, "r");
    assert_eq!(p.2, "docs");
}

#[test]
fn parse_dst_url_blob_file_uses_parent_directory() {
    let u = "https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/rules_sync.json";
    let p = parse_github_dst(u).expect("dst");
    assert_eq!(p.0, "lulufoo");
    assert_eq!(p.1, "ai-software-dev");
    assert_eq!(p.2, "sys-prompt");
}

#[test]
fn parse_dst_url_blob_file_nested_directory() {
    let u = "https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/cursor-rule-guard/v1/cursor-rule-guard-design.md";
    let p = parse_github_dst(u).expect("dst");
    assert_eq!(p.2, "sys-prompt/cursor-rule-guard/v1");
}

#[test]
fn parse_dst_url_blob_must_not_keep_blob_main_prefix() {
    let u = "https://github.com/o/r/blob/main/sys-prompt/cursor-rule-guard/v1/foo.md";
    let p = parse_github_dst(u).expect("dst");
    assert!(!p.2.starts_with("blob/"));
    assert!(!p.2.contains("main/"));
}

#[test]
fn parse_dst_url_blob_directory_segment_keeps_full_path() {
    let u = "https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/cursor-rule-guard/v1";
    let p = parse_github_dst(u).expect("dst");
    assert_eq!(p.2, "sys-prompt/cursor-rule-guard/v1");
}
