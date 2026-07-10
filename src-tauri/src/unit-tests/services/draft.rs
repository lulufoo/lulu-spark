use super::*;
use std::fs;

use crate::test_support::TestSandbox;

fn with_cache<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    f();
}

#[test]
fn save_draft_writes_under_cache_dir() {
    with_cache(|| {
        let v = save_comment_draft(&json!({
            "common_path": "proj/note.md",
            "content": "hello"
        }));
        assert_eq!(v["ok"], true);
        let p = paths::draft_path("proj/note.md").expect("path");
        assert_eq!(fs::read_to_string(&p).expect("read"), "hello");
    });
}

#[test]
fn save_note_draft_writes_under_drafts_notes_and_reads_back() {
    with_cache(|| {
        let temp_id = "tmp-abc123";
        save_note_draft(temp_id, "note body").expect("save");
        let p = paths::notes_draft_path(temp_id).expect("path");
        let cache = paths::cache_dir().expect("cache");
        assert_eq!(p, cache.join("drafts").join("notes").join(temp_id));
        assert_eq!(fs::read_to_string(&p).expect("read"), "note body");
    });
}

#[test]
fn clear_note_draft_removes_file_after_success_or_empty_exit() {
    with_cache(|| {
        let temp_id = "tmp-clear-me";
        save_note_draft(temp_id, "will clear").expect("save");
        let p = paths::notes_draft_path(temp_id).expect("path");
        assert!(p.is_file());
        clear_note_draft(temp_id).expect("clear");
        assert!(!p.exists(), "draft must be gone after success/empty-exit clear");
    });
}

#[test]
fn note_draft_is_isolated_from_comment_draft() {
    with_cache(|| {
        let comment = save_comment_draft(&json!({
            "common_path": "proj/existing.md",
            "content": "comment draft stays"
        }));
        assert_eq!(comment["ok"], true);
        let comment_path = paths::draft_path("proj/existing.md").expect("comment path");
        let before = fs::read_to_string(&comment_path).expect("read comment");

        save_note_draft("tmp-iso", "notes only").expect("save note");

        assert_eq!(
            fs::read_to_string(&comment_path).expect("reread comment"),
            before,
            "writing notes draft must not touch comment draft files"
        );
        let note_path = paths::notes_draft_path("tmp-iso").expect("note path");
        assert!(note_path.starts_with(paths::cache_dir().expect("cache").join("drafts").join("notes")));
        assert_ne!(note_path, comment_path);
    });
}

#[test]
fn note_draft_api_is_crash_buffer_not_product_draft_list() {
    with_cache(|| {
        // Crash-buffer API: write + clear only; no durable draft index / list artifact.
        let temp_id = "tmp-buffer";
        save_note_draft(temp_id, "buffered").expect("save");
        let notes_dir = paths::cache_dir()
            .expect("cache")
            .join("drafts")
            .join("notes");
        assert!(notes_dir.join(temp_id).is_file());
        assert!(
            !notes_dir.join("index.json").exists(),
            "notes draft must not introduce a product draft list/index"
        );
        clear_note_draft(temp_id).expect("clear");
        assert!(!notes_dir.join(temp_id).exists());
    });
}

#[test]
fn note_draft_retained_when_clear_not_called_simulating_archive_failure() {
    with_cache(|| {
        let temp_id = "tmp-retry";
        save_note_draft(temp_id, "retry me").expect("save");
        // Archive failure path: caller must not clear — draft stays for retry.
        let p = paths::notes_draft_path(temp_id).expect("path");
        assert!(p.is_file());
        assert_eq!(fs::read_to_string(&p).expect("read"), "retry me");
    });
}
