use std::fs;

use serde_json::json;

use super::resolve_source_path;
use crate::test_support::TestSandbox;

/// Read-only system file that exists outside Spark's roots.
const OUTSIDE_FILE: &str = "/etc/hosts";

#[test]
fn resolve_source_path_accepts_file_outside_spark_roots() {
    let resolved = resolve_source_path(OUTSIDE_FILE, u64::MAX).expect("resolve");
    assert!(resolved.is_absolute());
}

#[test]
fn resolve_source_path_rejects_empty_and_relative() {
    assert_eq!(resolve_source_path("  ", 10).unwrap_err()["_status"], json!(400));
    assert_eq!(resolve_source_path("a/b.md", 10).unwrap_err()["_status"], json!(400));
}

#[test]
fn resolve_source_path_rejects_missing_file() {
    let err = resolve_source_path("/var/empty/lulu-spark-missing.md", 10).unwrap_err();
    assert_eq!(err["_status"], json!(404));
}

#[test]
fn resolve_source_path_rejects_directory_and_oversize() {
    let sandbox = TestSandbox::new();
    let dir = sandbox.cache_dir().join("source_path_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let err = resolve_source_path(dir.to_str().unwrap(), 10).unwrap_err();
    assert_eq!(err["_status"], json!(400));
    let file = dir.join("big.md");
    fs::write(&file, b"0123456789A").expect("write");
    let err = resolve_source_path(file.to_str().unwrap(), 10).unwrap_err();
    assert_eq!(err["_status"], json!(413));
    assert!(resolve_source_path(file.to_str().unwrap(), 11).is_ok());
}
