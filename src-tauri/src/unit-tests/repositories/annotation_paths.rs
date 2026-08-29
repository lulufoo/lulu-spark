use super::*;
use std::fs;

#[test]
fn md_common_path_maps_to_annotations_json() {
    let dir = tempfile::tempdir().expect("tmp");
    let notes = dir.path().join("notes");
    fs::create_dir_all(notes.join("annotations/ai")).expect("mkdir");
    let p = annotation_json_path(&notes, "ai/note.md").expect("path");
    assert!(p.ends_with("annotations/ai/note.json"));
}

#[test]
fn resolves_when_topic_annotation_dir_missing() {
    let dir = tempfile::tempdir().expect("tmp");
    let notes = dir.path().join("notes");
    fs::create_dir_all(notes.join("annotations")).expect("mkdir");
    let p = annotation_json_path(&notes, "inbox/new-note.md").expect("path");
    assert!(p.ends_with("annotations/inbox/new-note.json"));
    assert!(!p.parent().expect("parent").exists());
}

#[test]
fn rejects_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).expect("mkdir");
    assert!(annotation_json_path(&notes, "../x.md").is_none());
}

#[test]
fn rejects_empty_and_absolute_common_path() {
    let dir = tempfile::tempdir().expect("tmp");
    let notes = dir.path().join("notes");
    fs::create_dir_all(notes.join("annotations")).expect("mkdir");
    assert!(annotation_json_path(&notes, "").is_none());
    assert!(annotation_json_path(&notes, "  ").is_none());
    assert!(annotation_json_path(&notes, "/ai/note.md").is_none());
}

#[test]
fn non_md_common_path_appends_json_suffix() {
    let dir = tempfile::tempdir().expect("tmp");
    let notes = dir.path().join("notes");
    fs::create_dir_all(notes.join("annotations")).expect("mkdir");
    let p = annotation_json_path(&notes, "inbox/draft").expect("path");
    assert!(p.ends_with("annotations/inbox/draft.json"));
}

#[test]
fn returns_none_when_notes_root_missing() {
    let dir = tempfile::tempdir().expect("tmp");
    let notes = dir.path().join("no-such-notes");
    assert!(annotation_json_path(&notes, "ai/note.md").is_none());
}
