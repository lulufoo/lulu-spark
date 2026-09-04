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
fn read_defaults_to_fifty_lines_and_reports_remaining() {
    let (fence, read_root, _scratch) = live_fence();
    let body = (1..=51)
        .map(|n| format!("line-{n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let path = read_root.join("paged.txt");
    fs::write(&path, body).expect("write");

    let first = host::call("read", &json!({ "path": path.to_string_lossy() }), &fence);
    assert!(!first.is_error, "{}", first.content);
    let first_body: serde_json::Value = serde_json::from_str(&first.content).expect("read json");
    assert_eq!(first_body["offset"], 1);
    assert_eq!(first_body["limit"], 50);
    assert_eq!(first_body["remaining_lines"], 1);
    let first_text = first_body["content"].as_str().expect("content");
    assert!(first_text.starts_with("1:line-1\n"));
    assert!(first_text.ends_with("50:line-50"));
    assert_eq!(
        first_body.as_object().expect("object").keys().count(),
        4
    );

    let last = host::call(
        "read",
        &json!({
            "path": path.to_string_lossy(),
            "offset": 51,
            "limit": 50
        }),
        &fence,
    );
    assert!(!last.is_error, "{}", last.content);
    let last_body: serde_json::Value = serde_json::from_str(&last.content).expect("read json");
    assert_eq!(last_body["offset"], 51);
    assert_eq!(last_body["limit"], 50);
    assert_eq!(last_body["remaining_lines"], 0);
    assert_eq!(last_body["content"], "51:line-51");
    assert_eq!(last_body.as_object().expect("object").keys().count(), 4);

    let past_end = host::call(
        "read",
        &json!({
            "path": path.to_string_lossy(),
            "offset": 52,
            "limit": 50
        }),
        &fence,
    );
    assert!(!past_end.is_error, "{}", past_end.content);
    let past_body: serde_json::Value = serde_json::from_str(&past_end.content).expect("read json");
    assert_eq!(past_body["offset"], 52);
    assert_eq!(past_body["limit"], 50);
    assert_eq!(past_body["remaining_lines"], 0);
    assert_eq!(past_body["content"], "");
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
    let body: serde_json::Value = serde_json::from_str(&read.content).expect("read json");
    assert_eq!(body["offset"], 2);
    assert_eq!(body["limit"], 1);
    assert_eq!(body["remaining_lines"], 1);
    assert_eq!(body["content"], "2:find-me");

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

#[test]
fn write_and_edit_succeed_on_exact_staged_file() {
    let read_root = unique_dir("read-staged");
    let scratch_parent = unique_dir("scratch-parent-staged");
    let file = read_root.join("note.md");
    fs::write(&file, "hello").expect("seed");
    let fence = PathFence {
        read_allow: vec![read_root.clone()],
        read_deny: vec![read_root.join(".git")],
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(scratch_parent.clone()),
    }
    .with_session_writes("sess_fs", [file.to_string_lossy()])
    .expect("writes");

    let other = read_root.join("other.md");
    let denied = host::call(
        "write",
        &json!({ "path": other.to_string_lossy(), "content": "nope" }),
        &fence,
    );
    assert!(denied.is_error);
    assert!(!other.exists());

    let wrote = host::call(
        "write",
        &json!({ "path": file.to_string_lossy(), "content": "hello world" }),
        &fence,
    );
    assert!(!wrote.is_error, "{}", wrote.content);
    assert_eq!(fs::read_to_string(&file).expect("read"), "hello world");

    let edited = host::call(
        "edit",
        &json!({
            "path": file.to_string_lossy(),
            "old_text": "world",
            "new_text": "staged"
        }),
        &fence,
    );
    assert!(!edited.is_error, "{}", edited.content);
    assert_eq!(fs::read_to_string(&file).expect("read"), "hello staged");

    let pad = scratch_parent.join("sess_fs").join("pad.md");
    let scratch = host::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "scratch" }),
        &fence,
    );
    assert!(!scratch.is_error, "{}", scratch.content);
}
