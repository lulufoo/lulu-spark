use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::agent::tools::host;
use crate::services::path_fence::PathFence;

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "wb-fs-tools-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos(),
        label
    ));
    fs::create_dir_all(&dir).expect("create unique dir");
    dir
}

fn live_fence() -> (PathFence, PathBuf, PathBuf) {
    let read_root = unique_dir("read");
    let scratch_parent = unique_dir("scratch-parent");
    let fence = PathFence {
        read_allow: vec![read_root.clone()],
        read_deny: vec![read_root.join(".git")],
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(scratch_parent.clone()),
    }
    .with_session_scratch("sess_fs")
    .expect("scratch");
    (fence, read_root, scratch_parent.join("sess_fs"))
}

#[test]
fn catalog_exposes_the_four_host_file_tools() {
    let catalog = host::catalog();
    for name in ["grep", "read", "write", "edit"] {
        assert!(catalog.contains(name), "missing {name}");
        assert!(host::is_builtin(name), "{name}");
    }
    for name in ["stage", "list_staged", "get_staged"] {
        assert!(!catalog.contains(name), "stage moved out of host: {name}");
        assert!(!host::is_builtin(name), "{name}");
    }
    assert!(catalog.is_mutating("write"));
    assert!(catalog.is_mutating("edit"));
    assert!(!catalog.is_mutating("read"));
    assert!(!catalog.is_mutating("grep"));
}

#[test]
fn grep_and_read_stay_inside_the_read_fence() {
    let (fence, read_root, _scratch) = live_fence();
    fs::write(read_root.join("a.txt"), "alpha\nfind-me\nomega\n").expect("write");
    let grep = host::call(
        "grep",
        &json!({ "pattern": "find-me", "path": read_root.to_string_lossy() }),
        &fence,
    );
    assert!(!grep.is_error, "{}", grep.content);
    assert!(grep.content.contains("find-me"));

    let read = host::call(
        "read",
        &json!({
            "path": read_root.join("a.txt").to_string_lossy(),
            "offset": 2,
            "limit": 1
        }),
        &fence,
    );
    assert!(!read.is_error, "{}", read.content);
    assert_eq!(read.content, "2:find-me");

    let denied = host::call("read", &json!({ "path": "/etc/hosts" }), &fence);
    assert!(denied.is_error);
    assert!(denied.content.contains("outside the read fence"));
}

#[test]
fn write_and_edit_are_scratch_only() {
    let (fence, read_root, scratch) = live_fence();
    let outside = read_root.join("todo.md");
    let denied = host::call(
        "write",
        &json!({ "path": outside.to_string_lossy(), "content": "nope" }),
        &fence,
    );
    assert!(denied.is_error);
    assert!(!outside.exists());

    let pad = scratch.join("pad.md");
    let wrote = host::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "hello world" }),
        &fence,
    );
    assert!(!wrote.is_error, "{}", wrote.content);
    assert_eq!(fs::read_to_string(&pad).expect("read pad"), "hello world");

    let edited = host::call(
        "edit",
        &json!({
            "path": pad.to_string_lossy(),
            "old_text": "world",
            "new_text": "scratch"
        }),
        &fence,
    );
    assert!(!edited.is_error, "{}", edited.content);
    assert_eq!(fs::read_to_string(&pad).expect("read pad"), "hello scratch");

    let twice = host::call(
        "edit",
        &json!({
            "path": pad.to_string_lossy(),
            "old_text": "hello",
            "new_text": "x"
        }),
        &fence,
    );
    assert!(!twice.is_error);
    let twice_again = host::call(
        "edit",
        &json!({
            "path": pad.to_string_lossy(),
            "old_text": "x",
            "new_text": "y"
        }),
        &fence,
    );
    assert!(!twice_again.is_error);
    let dup = host::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "aa aa" }),
        &fence,
    );
    assert!(!dup.is_error);
    let many = host::call(
        "edit",
        &json!({
            "path": pad.to_string_lossy(),
            "old_text": "aa",
            "new_text": "bb"
        }),
        &fence,
    );
    assert!(many.is_error);
    assert!(many.content.contains("more than once"));
}
