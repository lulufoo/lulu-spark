use std::fs;
use std::path::PathBuf;

use crate::services::file_export::copy_into_dir_versioned;
use crate::test_support::TestSandbox;

const ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn source_file(sandbox: &TestSandbox, body: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("export_source");
    fs::create_dir_all(&dir).expect("mkdir source");
    let path = dir.join("source.md");
    fs::write(&path, body).expect("write source");
    path
}

/// A destination outside the sandbox, so it never touches Spark's own directories.
fn dest_outside(outside: &tempfile::TempDir, name: &str) -> PathBuf {
    outside.path().join(name)
}

#[test]
fn copy_writes_id_named_file_and_creates_missing_dir() {
    let sandbox = TestSandbox::new();
    let outside = tempfile::tempdir().expect("outside dir");
    let src = source_file(&sandbox, "# body");
    let dest = dest_outside(&outside, "ws/session-1");
    let out = copy_into_dir_versioned(&src, dest.to_str().unwrap(), ID, "md").expect("copy");
    assert_eq!(out.file_name().unwrap().to_str().unwrap(), format!("{ID}.md"));
    assert_eq!(fs::read_to_string(&out).unwrap(), "# body");
}

#[test]
fn copy_with_empty_ext_drops_the_dot() {
    let sandbox = TestSandbox::new();
    let outside = tempfile::tempdir().expect("outside dir");
    let src = source_file(&sandbox, "x");
    let dest = dest_outside(&outside, "ws/bare");
    let dest = dest.to_str().unwrap();
    let first = copy_into_dir_versioned(&src, dest, "doc", "").expect("v0");
    let second = copy_into_dir_versioned(&src, dest, "doc", "").expect("v1");
    assert_eq!(first.file_name().unwrap().to_str().unwrap(), "doc");
    assert_eq!(second.file_name().unwrap().to_str().unwrap(), "doc-v1");
}

#[test]
fn copy_never_overwrites_and_bumps_version() {
    let sandbox = TestSandbox::new();
    let outside = tempfile::tempdir().expect("outside dir");
    let src = source_file(&sandbox, "first");
    let dest = dest_outside(&outside, "ws/versions");
    let dest = dest.to_str().unwrap();
    let first = copy_into_dir_versioned(&src, dest, ID, "md").expect("v0");
    fs::write(&first, "edited by agent").unwrap();
    let second = copy_into_dir_versioned(&src, dest, ID, "md").expect("v1");
    let third = copy_into_dir_versioned(&src, dest, ID, "md").expect("v2");
    assert_eq!(second.file_name().unwrap().to_str().unwrap(), format!("{ID}-v1.md"));
    assert_eq!(third.file_name().unwrap().to_str().unwrap(), format!("{ID}-v2.md"));
    assert_eq!(fs::read_to_string(&first).unwrap(), "edited by agent");
    assert_eq!(fs::read_to_string(&second).unwrap(), "first");
}

#[test]
fn copy_rejects_relative_and_parent_dir_paths() {
    let sandbox = TestSandbox::new();
    let outside = tempfile::tempdir().expect("outside dir");
    let src = source_file(&sandbox, "x");
    let relative = copy_into_dir_versioned(&src, "relative/dir", ID, "md").unwrap_err();
    assert_eq!(relative["_status"], 400);
    let empty = copy_into_dir_versioned(&src, "  ", ID, "md").unwrap_err();
    assert_eq!(empty["_status"], 400);
    let dotdot = dest_outside(&outside, "a/../b");
    let parent = copy_into_dir_versioned(&src, dotdot.to_str().unwrap(), ID, "md").unwrap_err();
    assert_eq!(parent["_status"], 400);
}

#[test]
fn copy_accepts_dir_outside_spark_base_dirs() {
    let sandbox = TestSandbox::new();
    let src = source_file(&sandbox, "x");
    let dir = tempfile::tempdir().expect("outside dir");
    let dest = dir.path().join("exports");
    let out = copy_into_dir_versioned(&src, dest.to_str().unwrap(), ID, "md").expect("copy");
    assert!(out.starts_with(dest.canonicalize().unwrap()));
}

#[test]
fn copy_accepts_dir_under_cache_outside_data() {
    let sandbox = TestSandbox::new();
    let src = source_file(&sandbox, "x");
    let dest = sandbox.cache_dir().join("agent-workspace").join("session-1");
    let out = copy_into_dir_versioned(&src, dest.to_str().unwrap(), ID, "md").expect("copy");
    assert_eq!(fs::read_to_string(&out).unwrap(), "x");
    assert!(out.starts_with(dest.canonicalize().unwrap()));
}

#[test]
fn copy_accepts_dir_under_config_dir() {
    let sandbox = TestSandbox::new();
    let src = source_file(&sandbox, "x");
    let dest = sandbox.config_dir().join("exports");
    let out = copy_into_dir_versioned(&src, dest.to_str().unwrap(), ID, "md").expect("copy");
    assert!(out.starts_with(dest.canonicalize().unwrap()));
}

#[test]
fn copy_rejects_dir_inside_data_dir() {
    let sandbox = TestSandbox::new();
    let src = source_file(&sandbox, "x");
    let in_data = sandbox.data_dir().join("notes").join("raw").join("inbox");
    let err = copy_into_dir_versioned(&src, in_data.to_str().unwrap(), ID, "md").unwrap_err();
    assert_eq!(err["_status"], 403);
    assert!(!in_data.exists(), "must not create a dir under Data");
}

#[cfg(unix)]
#[test]
fn copy_rejects_symlink_into_data_dir() {
    let sandbox = TestSandbox::new();
    let outside = tempfile::tempdir().expect("outside dir");
    let src = source_file(&sandbox, "x");
    let target = sandbox.data_dir().join("notes");
    fs::create_dir_all(&target).unwrap();
    let link = dest_outside(&outside, "link-to-data");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let dest = link.join("sub");
    let err = copy_into_dir_versioned(&src, dest.to_str().unwrap(), ID, "md").unwrap_err();
    assert_eq!(err["_status"], 403);
}

#[test]
fn copy_rejects_unsafe_stem() {
    let sandbox = TestSandbox::new();
    let outside = tempfile::tempdir().expect("outside dir");
    let src = source_file(&sandbox, "x");
    let dest = dest_outside(&outside, "ws/stem");
    for bad in ["", "../x", "a/b", "a\\b"] {
        let err = copy_into_dir_versioned(&src, dest.to_str().unwrap(), bad, "md").unwrap_err();
        assert_eq!(err["_status"], 400, "stem {bad:?}");
    }
}
