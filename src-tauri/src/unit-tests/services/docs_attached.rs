use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::*;

const DOC: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn md_dir(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
    let dir = tmp.path().join(name);
    fs::create_dir_all(&dir).expect("md dir");
    dir
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent");
    }
    fs::write(path, bytes).expect("write");
}

#[test]
fn attached_paths_use_the_document_owned_layout() {
    let md = Path::new("/notes/raw/inbox/topic");
    assert_eq!(
        attached_dir(md, DOC).expect("dir"),
        md.join(".attached").join(DOC)
    );
    assert_eq!(
        attached_file(md, DOC, "overview.png").expect("file"),
        md.join(".attached").join(DOC).join("overview.png")
    );
    assert_eq!(
        attached_rel(DOC, "overview.png").expect("rel"),
        format!(".attached/{DOC}/overview.png")
    );
}

#[test]
fn attached_paths_refuse_a_bad_doc_id_or_basename() {
    let md = Path::new("/notes");
    for bad_id in ["", "abc", "../x", "AA", "a/b"] {
        let err = attached_dir(md, bad_id).expect_err(bad_id);
        assert_eq!(err["_status"], json!(400), "{bad_id}");
        assert_eq!(err["error"], "Invalid id");
    }
    for bad_name in ["", ".", "..", "../x.png", "a/b.png", "a\\b.png", "foo..png"] {
        let err = attached_file(md, DOC, bad_name).expect_err(bad_name);
        assert_eq!(err["_status"], json!(400), "{bad_name}");
        assert_eq!(err["error"], "Invalid path");
        assert!(attached_rel(DOC, bad_name).is_err(), "{bad_name}");
    }
}

#[test]
fn copy_attached_dir_copies_files_and_leaves_the_source() {
    let tmp = tempfile::tempdir().expect("tmp");
    let src = md_dir(&tmp, "src");
    let dest = md_dir(&tmp, "dest");
    write_file(&src.join(".attached").join(DOC).join("a.png"), b"png-a");
    write_file(&src.join(".attached").join(DOC).join("b.jpg"), b"jpg-b");

    let copied = copy_attached_dir(&src, &dest, DOC).expect("copy");
    assert_eq!(
        copied.as_deref(),
        Some(dest.join(".attached").join(DOC).as_path())
    );
    assert_eq!(
        fs::read(dest.join(".attached").join(DOC).join("a.png")).unwrap(),
        b"png-a"
    );
    assert_eq!(
        fs::read(dest.join(".attached").join(DOC).join("b.jpg")).unwrap(),
        b"jpg-b"
    );
    assert_eq!(
        fs::read(src.join(".attached").join(DOC).join("a.png")).unwrap(),
        b"png-a"
    );
}

#[test]
fn copy_attached_dir_is_a_noop_when_the_source_is_missing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let src = md_dir(&tmp, "src");
    let dest = md_dir(&tmp, "dest");
    assert_eq!(copy_attached_dir(&src, &dest, DOC).expect("copy"), None);
    assert!(!dest.join(".attached").exists());
}

#[test]
fn copy_attached_dir_is_409_when_the_dest_already_exists() {
    let tmp = tempfile::tempdir().expect("tmp");
    let src = md_dir(&tmp, "src");
    let dest = md_dir(&tmp, "dest");
    write_file(&src.join(".attached").join(DOC).join("a.png"), b"new");
    write_file(&dest.join(".attached").join(DOC).join("a.png"), b"old");
    let err = copy_attached_dir(&src, &dest, DOC).expect_err("collision");
    assert_eq!(err["_status"], json!(409));
    assert_eq!(err["error"], "File already exists");
    assert_eq!(
        fs::read(dest.join(".attached").join(DOC).join("a.png")).unwrap(),
        b"old"
    );
}

#[test]
fn remove_attached_dir_deletes_the_doc_folder_and_an_empty_parent() {
    let tmp = tempfile::tempdir().expect("tmp");
    let md = md_dir(&tmp, "notes");
    write_file(&md.join(".attached").join(DOC).join("a.png"), b"png");
    remove_attached_dir(&md, DOC).expect("remove");
    assert!(!md.join(".attached").join(DOC).exists());
    assert!(!md.join(".attached").exists());
}

#[test]
fn remove_attached_dir_keeps_a_sibling_doc_folder() {
    let tmp = tempfile::tempdir().expect("tmp");
    let md = md_dir(&tmp, "notes");
    write_file(&md.join(".attached").join(DOC).join("a.png"), b"a");
    write_file(&md.join(".attached").join(OTHER).join("b.png"), b"b");
    remove_attached_dir(&md, DOC).expect("remove");
    assert!(!md.join(".attached").join(DOC).exists());
    assert_eq!(
        fs::read(md.join(".attached").join(OTHER).join("b.png")).unwrap(),
        b"b"
    );
}

#[test]
fn remove_attached_dir_is_a_noop_when_missing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let md = md_dir(&tmp, "notes");
    remove_attached_dir(&md, DOC).expect("remove");
    assert!(!md.join(".attached").exists());
}

#[test]
fn orphan_attachment_names_lists_unreferenced_files() {
    let tmp = tempfile::tempdir().expect("tmp");
    let md = md_dir(&tmp, "notes");
    write_file(&md.join(".attached").join(DOC).join("keep.png"), b"k");
    write_file(&md.join(".attached").join(DOC).join("gone.png"), b"g");
    let names = orphan_attachment_names(&md, DOC, &["![x](.attached/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/keep.png)"]);
    assert_eq!(names, vec!["gone.png"]);
}

#[test]
fn orphan_attachment_names_keeps_a_translation_only_ref() {
    let tmp = tempfile::tempdir().expect("tmp");
    let md = md_dir(&tmp, "notes");
    write_file(&md.join(".attached").join(DOC).join("zh-only.png"), b"z");
    let names = orphan_attachment_names(
        &md,
        DOC,
        &["# raw\n", "![图](.attached/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/zh-only.png)"],
    );
    assert!(names.is_empty(), "{names:?}");
}
