use super::*;
use std::fs;

#[test]
fn md_common_path_maps_to_annotations_json() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("annotations/ai")).expect("mkdir");
    let p = annotation_json_path(&corpus, "ai/note.md").expect("path");
    assert!(p.ends_with("annotations/ai/note.json"));
}

#[test]
fn resolves_when_topic_annotation_dir_missing() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("annotations")).expect("mkdir");
    let p = annotation_json_path(&corpus, "inbox/new-note.md").expect("path");
    assert!(p.ends_with("annotations/inbox/new-note.json"));
    assert!(!p.parent().expect("parent").exists());
}

#[test]
fn rejects_traversal() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    assert!(annotation_json_path(&corpus, "../x.md").is_none());
}

#[test]
fn rejects_empty_and_absolute_common_path() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("annotations")).expect("mkdir");
    assert!(annotation_json_path(&corpus, "").is_none());
    assert!(annotation_json_path(&corpus, "  ").is_none());
    assert!(annotation_json_path(&corpus, "/ai/note.md").is_none());
}

#[test]
fn non_md_common_path_appends_json_suffix() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("annotations")).expect("mkdir");
    let p = annotation_json_path(&corpus, "inbox/draft").expect("path");
    assert!(p.ends_with("annotations/inbox/draft.json"));
}

#[test]
fn returns_none_when_corpus_root_missing() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("no-such-corpus");
    assert!(annotation_json_path(&corpus, "ai/note.md").is_none());
}
