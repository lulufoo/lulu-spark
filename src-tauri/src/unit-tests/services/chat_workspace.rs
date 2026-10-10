use std::fs;

use crate::services::chat_workspace::list_session_workspace_files;
use crate::services::path_fence::{stored_path, PathFence};

fn fence_with_parent(parent: &std::path::Path) -> PathFence {
    PathFence {
        scratch_parent: Some(parent.to_path_buf()),
        ..PathFence::default()
    }
}

#[test]
fn missing_scratch_dir_returns_empty() {
    let tmp = tempfile::tempdir().expect("tmp");
    let fence = fence_with_parent(tmp.path());
    let files = list_session_workspace_files("spark_chat_abc", &fence).expect("list");
    assert!(files.is_empty());
}

#[test]
fn lists_nested_regular_files_and_skips_hidden() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path().join("spark_chat_abc");
    fs::create_dir_all(root.join("nested")).expect("dir");
    fs::write(root.join("a.md"), "a").expect("a");
    fs::write(root.join("nested").join("b.md"), "b").expect("b");
    fs::write(root.join(".hidden.md"), "x").expect("hidden");
    let fence = fence_with_parent(tmp.path());
    let files = list_session_workspace_files("spark_chat_abc", &fence).expect("list");
    let titles: Vec<_> = files.iter().map(|f| f.title.as_str()).collect();
    assert_eq!(titles, vec!["a.md", "nested/b.md"]);
    assert_eq!(files[0].path, stored_path(root.join("a.md")).to_string_lossy());
}

#[test]
fn session_roots_are_isolated() {
    let tmp = tempfile::tempdir().expect("tmp");
    fs::create_dir_all(tmp.path().join("sess_a")).expect("a");
    fs::create_dir_all(tmp.path().join("sess_b")).expect("b");
    fs::write(tmp.path().join("sess_a").join("only-a.md"), "a").expect("write a");
    fs::write(tmp.path().join("sess_b").join("only-b.md"), "b").expect("write b");
    let fence = fence_with_parent(tmp.path());
    let a = list_session_workspace_files("sess_a", &fence).expect("a");
    let b = list_session_workspace_files("sess_b", &fence).expect("b");
    assert_eq!(a.iter().map(|f| f.title.as_str()).collect::<Vec<_>>(), vec!["only-a.md"]);
    assert_eq!(b.iter().map(|f| f.title.as_str()).collect::<Vec<_>>(), vec!["only-b.md"]);
}
