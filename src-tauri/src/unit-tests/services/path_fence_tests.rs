use std::fs;
use std::path::PathBuf;

use crate::services::path_fence::{
    require_absolute, sanitize_session_segment, stored_path, validate_stage_file, PathFence,
};

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "wb-path-fence-{}-{}-{}",
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

fn fence_with(root: PathBuf, scratch_parent: PathBuf) -> PathFence {
    PathFence {
        read_allow: vec![root.clone()],
        read_deny: vec![root.join(".git")],
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(scratch_parent),
    }
}

#[test]
fn read_allow_accepts_root_file_and_denies_git() {
    let root = unique_dir("read");
    let file = root.join("note.md");
    fs::write(&file, "hi").expect("write");
    let fence = fence_with(root.clone(), unique_dir("scratch-parent"));
    assert!(fence.allows_read(&file));
    assert!(fence.allows_read(&root));
    assert!(!fence.allows_read(&root.join(".git").join("config")));
    assert!(!fence.allows_read(&root.join("..").join("outside.txt")));
}

#[test]
fn write_is_empty_until_session_scratch_is_resolved() {
    let root = unique_dir("wb");
    let scratch_parent = unique_dir("agent-scratch");
    let fence = fence_with(root.clone(), scratch_parent.clone());
    assert!(fence.write_allow.is_empty());
    assert!(!fence.allows_write(&root.join("todo.md")));

    let live = fence
        .with_session_scratch("sess_abc")
        .expect("session id is legal");
    let scratch = scratch_parent.join("sess_abc");
    assert_eq!(live.write_allow, vec![scratch.clone()]);
    assert!(live.allows_write(&scratch.join("pad.md")));
    assert!(live.allows_read(&scratch.join("pad.md")));
    assert!(!live.allows_write(&root.join("todo.md")));
}

#[test]
fn blacklist_wins_over_allow() {
    let root = unique_dir("deny");
    let fence = PathFence {
        read_allow: vec![root.clone()],
        read_deny: vec![root.join("secret")],
        write_allow: vec![root.clone()],
        write_deny: vec![],
        scratch_parent: None,
    };
    assert!(fence.allows_read(&root.join("ok.txt")));
    assert!(!fence.allows_read(&root.join("secret").join("a.txt")));
    assert!(!fence.allows_write(&root.join("secret").join("a.txt")));
}

#[test]
fn require_absolute_rejects_relative_and_normalizes_dotdot() {
    assert!(require_absolute("relative/file").is_err());
    assert!(require_absolute("").is_err());
    let normalized = require_absolute("/tmp/root/sub/../leaf.txt").expect("abs");
    assert_eq!(normalized, PathBuf::from("/tmp/root/leaf.txt"));
}

#[test]
fn write_allows_not_yet_created_file_when_root_is_canonical() {
    let parent = unique_dir("scratch-parent");
    let parent_canon = parent.canonicalize().unwrap_or(parent.clone());
    let fence = fence_with(unique_dir("wb"), parent_canon)
        .with_session_scratch("sess_abc")
        .expect("scratch");
    let requested = parent.join("sess_abc").join("pad.md");
    assert!(
        fence.allows_write(&requested),
        "write root={:?} requested={requested:?}",
        fence.write_allow
    );
}

#[test]
fn sanitize_session_segment_rejects_empty_slash_and_overlong() {
    assert!(sanitize_session_segment("sess_abc").is_ok());
    assert!(sanitize_session_segment("").is_err());
    assert!(sanitize_session_segment("../escape").is_err());
    assert!(sanitize_session_segment(&"a".repeat(81)).is_err());
}

#[test]
fn validate_stage_file_accepts_regular_file_and_rejects_dir_deny() {
    let root = unique_dir("stage-file");
    let file = root.join("ok.md");
    fs::write(&file, "hi").expect("write");
    fs::create_dir_all(root.join(".git")).expect("git dir");
    fs::write(root.join(".git").join("config"), "x").expect("git file");
    let fence = fence_with(root.clone(), unique_dir("scratch-parent"));
    let canon = validate_stage_file(&file.to_string_lossy(), &fence).expect("file");
    assert_eq!(canon, stored_path(file));
    assert!(validate_stage_file(&root.to_string_lossy(), &fence)
        .unwrap_err()
        .contains("regular file"));
    assert!(validate_stage_file("rel.md", &fence).is_err());
    assert!(
        validate_stage_file(&root.join(".git").join("config").to_string_lossy(), &fence)
            .unwrap_err()
            .contains("read fence")
    );
}

#[test]
fn session_writes_add_exact_file_and_keep_scratch() {
    let root = unique_dir("wb-writes");
    let file = root.join("note.md");
    fs::write(&file, "hi").expect("write");
    let scratch_parent = unique_dir("agent-scratch");
    let fence = fence_with(root.clone(), scratch_parent.clone())
        .with_session_writes("sess_abc", [file.to_string_lossy()])
        .expect("writes");
    let scratch = scratch_parent.join("sess_abc");
    assert!(fence.allows_write(&scratch.join("pad.md")));
    assert!(fence.allows_write(&file));
    assert!(!fence.allows_write(&root.join("other.md")));
}

#[test]
fn session_writes_are_isolated_and_skip_invalid_staged() {
    let root = unique_dir("iso");
    let file_a = root.join("a.md");
    let file_b = root.join("b.md");
    fs::write(&file_a, "a").expect("a");
    fs::write(&file_b, "b").expect("b");
    let parent = unique_dir("scratch-iso");
    let base = fence_with(root.clone(), parent);
    let fence_a = base
        .with_session_writes("sess_a", [file_a.to_string_lossy()])
        .expect("a");
    let fence_b = base
        .with_session_writes("sess_b", [file_b.to_string_lossy()])
        .expect("b");
    assert!(fence_a.allows_write(&file_a));
    assert!(!fence_a.allows_write(&file_b));
    assert!(fence_b.allows_write(&file_b));
    assert!(!fence_b.allows_write(&file_a));
    let skipped = base
        .with_session_writes("sess_a", [root.to_string_lossy()])
        .expect("skip dir");
    assert!(!skipped.allows_write(&file_a));
}
