use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::services::agent::fs_tools;
use crate::services::agent::path_fence::PathFence;
use crate::services::agent::r#loop;
use crate::services::agent::session;
use crate::test_support::TestSandbox;

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
    let catalog = fs_tools::catalog();
    for name in ["grep", "read", "write", "edit"] {
        assert!(catalog.contains(name), "missing {name}");
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
    let grep = fs_tools::call(
        "grep",
        &json!({ "pattern": "find-me", "path": read_root.to_string_lossy() }),
        &fence,
    );
    assert!(!grep.is_error, "{}", grep.content);
    assert!(grep.content.contains("find-me"));

    let read = fs_tools::call(
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

    let denied = fs_tools::call(
        "read",
        &json!({ "path": "/etc/hosts" }),
        &fence,
    );
    assert!(denied.is_error);
    assert!(denied.content.contains("outside the read fence"));
}

#[test]
fn write_and_edit_are_scratch_only() {
    let (fence, read_root, scratch) = live_fence();
    let outside = read_root.join("todo.md");
    let denied = fs_tools::call(
        "write",
        &json!({ "path": outside.to_string_lossy(), "content": "nope" }),
        &fence,
    );
    assert!(denied.is_error);
    assert!(!outside.exists());

    let pad = scratch.join("pad.md");
    let wrote = fs_tools::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "hello world" }),
        &fence,
    );
    assert!(!wrote.is_error, "{}", wrote.content);
    assert_eq!(fs::read_to_string(&pad).expect("read pad"), "hello world");

    let edited = fs_tools::call(
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

    let twice = fs_tools::call(
        "edit",
        &json!({
            "path": pad.to_string_lossy(),
            "old_text": "hello",
            "new_text": "x"
        }),
        &fence,
    );
    assert!(!twice.is_error);
    let twice_again = fs_tools::call(
        "edit",
        &json!({
            "path": pad.to_string_lossy(),
            "old_text": "x",
            "new_text": "y"
        }),
        &fence,
    );
    assert!(!twice_again.is_error);
    let dup = fs_tools::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "aa aa" }),
        &fence,
    );
    assert!(!dup.is_error);
    let many = fs_tools::call(
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

const SECRET_BODY: &str = "SECRET_BODY_MUST_NOT_APPEAR";

fn with_live_chat<F: FnOnce(&PathFence, &str)>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    let sid = r#loop::create_chat_session_core().expect("chat")["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (fence, _, _) = live_fence();
    f(&fence, &sid);
}

fn as_json(content: &str) -> Value {
    serde_json::from_str(content).unwrap_or_else(|_| panic!("json: {content}"))
}

fn assert_path_only(entry: &Value, path: &str, title: &str) {
    let obj = entry.as_object().expect("object");
    assert!(obj.get("id").and_then(Value::as_str).is_some_and(|s| !s.is_empty()));
    assert_eq!(entry["path"], path);
    assert_eq!(entry["title"], title);
    for key in ["content", "body", "text"] {
        assert!(!obj.contains_key(key), "must not persist {key}");
    }
    assert!(!entry.to_string().contains(SECRET_BODY));
}

fn write_note(label: &str, name: &str) -> PathBuf {
    let path = unique_dir(label).join(name);
    fs::write(&path, SECRET_BODY).expect("write");
    path
}

#[test]
fn catalog_registers_stage_trio_off_cursor_ide_and_mobile() {
    let catalog = fs_tools::catalog();
    for name in ["grep", "read", "write", "edit", "stage", "list_staged", "get_staged"] {
        assert!(catalog.contains(name) && fs_tools::is_builtin(name), "{name}");
    }
    assert!(catalog.is_mutating("stage"));
    assert!(!catalog.is_mutating("list_staged") && !catalog.is_mutating("get_staged"));
    for (slot, channel) in [("cursor_ide", "cursor_ide"), ("workbench", "mobile")] {
        let table = crate::services::mcp_protocol_adapter::build_channel_tool_table(slot, channel)
            .expect("table");
        for name in ["stage", "list_staged", "get_staged"] {
            assert!(table.tools.iter().all(|t| t.name != name), "{name} on {channel}");
        }
    }
}

#[test]
fn stage_path_appends_default_title_without_body() {
    with_live_chat(|fence, sid| {
        let path = write_note("stage-default", "my-note.md");
        let path_str = path.to_string_lossy().to_string();
        let result = fs_tools::call("stage", &json!({ "path": path_str }), fence);
        assert!(!result.is_error && !result.content.contains(SECRET_BODY), "{}", result.content);
        let loaded = session::load_session(sid).expect("load");
        assert_eq!(loaded.staged.len(), 1);
        assert_path_only(&serde_json::to_value(&loaded.staged[0]).unwrap(), &path_str, "my-note");
        assert_path_only(&as_json(&result.content), &path_str, "my-note");
    });
}

#[test]
fn stage_uses_caller_title() {
    with_live_chat(|fence, sid| {
        let path = write_note("stage-title", "file.md");
        let path_str = path.to_string_lossy().to_string();
        let result = fs_tools::call("stage", &json!({ "path": path_str, "title": "Given Title" }), fence);
        assert!(!result.is_error, "{}", result.content);
        let loaded = session::load_session(sid).expect("load");
        assert_eq!((loaded.staged[0].title.as_str(), loaded.staged[0].path.as_str()), ("Given Title", path_str.as_str()));
    });
}

#[test]
fn list_staged_lists_only_current_session() {
    with_live_chat(|fence, sid_a| {
        let a = write_note("list-a", "a.md");
        assert!(!fs_tools::call("stage", &json!({"path": a.to_string_lossy(), "title": "A"}), fence).is_error);
        let sid_b = r#loop::create_chat_session_core().expect("b")["session_id"].as_str().unwrap().to_string();
        let b = write_note("list-b", "b.md");
        assert!(!fs_tools::call("stage", &json!({"path": b.to_string_lossy(), "title": "B"}), fence).is_error);
        let listed_b = as_json(&fs_tools::call("list_staged", &json!({}), fence).content);
        assert_eq!(listed_b.as_array().expect("b").len(), 1);
        assert_eq!(listed_b[0]["title"], "B");
        assert_ne!(sid_a, sid_b);
        r#loop::select_chat_session_core(sid_a).expect("select a");
        let listed_a = as_json(&fs_tools::call("list_staged", &json!({}), fence).content);
        assert_eq!(listed_a.as_array().expect("a")[0]["title"], "A");
        assert_eq!(listed_a.as_array().unwrap().len(), 1);
    });
}

#[test]
fn get_staged_returns_id_path_title_without_body() {
    with_live_chat(|fence, sid| {
        let path = write_note("get-staged", "doc.md");
        let path_str = path.to_string_lossy().to_string();
        assert!(!fs_tools::call("stage", &json!({ "path": path_str }), fence).is_error);
        let id = session::load_session(sid).expect("load").staged[0].id.clone();
        let got = fs_tools::call("get_staged", &json!({ "id": id }), fence);
        assert!(!got.is_error && !got.content.contains(SECRET_BODY), "{}", got.content);
        assert_path_only(&as_json(&got.content), &path_str, "doc");
    });
}

#[test]
fn stage_missing_path_fails_and_does_not_write() {
    with_live_chat(|fence, sid| {
        let result = fs_tools::call("stage", &json!({ "title": "no-path" }), fence);
        assert!(result.is_error && !result.content.contains(SECRET_BODY));
        assert!(session::load_session(sid).expect("load").staged.is_empty());
    });
}

#[test]
fn get_staged_missing_or_unknown_id_fails_without_body() {
    with_live_chat(|fence, _sid| {
        for args in [json!({}), json!({ "id": "stg_missing" })] {
            let result = fs_tools::call("get_staged", &args, fence);
            assert!(result.is_error && !result.content.contains(SECRET_BODY), "{}", result.content);
        }
    });
}
