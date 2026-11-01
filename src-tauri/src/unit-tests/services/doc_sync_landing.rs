use std::fs;

use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::services::doc_sync::tests::{id, write_at};

const PROJECT: &str = "inbox1";
const RAW: &str = "notes/202606190001-topic.md";
const EN: &str = "notes/202606190001-topic-en.md";

/// A workspace folder with the files the test lays down.
struct Folder {
    _tmp: TempDir,
    root: std::path::PathBuf,
}

impl Folder {
    fn new() -> Self {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path().join("ws");
        fs::create_dir_all(&root).expect("workspace folder");
        Self { _tmp: tmp, root }
    }

    fn file(&self, layer: &str, rel: &str) {
        write_at(&self.root.join(layer).join(PROJECT).join(rel), b"x");
    }
}

fn package(raw: &str, digest: Option<&str>, langs: &[&str]) -> Unpacked {
    Unpacked {
        raw_path: raw.to_string(),
        digest_path: digest.map(str::to_string),
        translations: langs.iter().map(|l| l.to_string()).collect(),
    }
}

fn row(n: u32, raw: &str, digest: Option<&str>, langs: &[&str]) -> DocRow {
    let mut row = DocRow::new(&id(n), raw, PROJECT);
    row.digest_path = digest.map(str::to_string);
    row.translations = json!(langs);
    row
}

/// Suffixes `a`, `b`, `c`, … in turn.
fn letters() -> impl FnMut() -> String {
    let mut next = b'a';
    move || {
        let letter = next as char;
        next += 1;
        letter.to_string()
    }
}

fn landing(raw: &str, digest: Option<&str>) -> Landing {
    Landing { raw_path: raw.to_string(), digest_path: digest.map(str::to_string) }
}

fn plan(
    folder: &Folder,
    package: &Unpacked,
    existing: Option<&DocRow>,
    others: &[DocRow],
) -> Result<Landing, String> {
    plan_landing(&folder.root, PROJECT, &id(1), package, existing, others, letters())
}

#[test]
fn a_package_whose_names_are_free_keeps_them() {
    let folder = Folder::new();
    folder.file("raw", "notes/another.md");
    folder.file("digest", "notes/another.md");

    let got = plan(&folder, &package(RAW, Some(RAW), &["en"]), None, &[row(2, "notes/another.md", None, &[])]);

    assert_eq!(got, Ok(landing(RAW, Some(RAW))));
}

#[test]
fn a_name_in_use_gets_a_suffix_in_front_of_the_md_and_the_translations_follow() {
    type Lay = &'static dyn Fn(&Folder);
    // (what is in use, the files that are there, the rows of the other documents)
    let cases: Vec<(&str, Lay, Vec<DocRow>)> = vec![
        ("body on disk, no row", &|f| f.file("raw", RAW), vec![]),
        ("body named by another row, no file", &|_| {}, vec![row(2, RAW, None, &[])]),
        ("translation on disk", &|f| f.file("raw", EN), vec![]),
        ("translation path named by another row", &|_| {}, vec![row(2, EN, None, &[])]),
        ("digest on disk", &|f| f.file("digest", RAW), vec![]),
        ("digest named by another row", &|_| {}, vec![row(2, "notes/z.md", Some(RAW), &[])]),
    ];
    for (what, lay, others) in cases {
        let folder = Folder::new();
        lay(&folder);

        let got = plan(&folder, &package(RAW, Some(RAW), &["en"]), None, &others);

        assert_eq!(
            got,
            Ok(landing("notes/202606190001-topic-a.md", Some("notes/202606190001-topic-a.md"))),
            "{what}"
        );
    }
}

#[test]
fn the_translation_of_another_document_is_in_the_way_of_a_body() {
    let folder = Folder::new();
    let others = [row(2, "notes/202606190002-topic.md", None, &["x"])]; // its translation is …-topic-x.md

    let got = plan(&folder, &package("notes/202606190002-topic-x.md", None, &[]), None, &others);

    assert_eq!(got, Ok(landing("notes/202606190002-topic-x-a.md", None)));
}

#[test]
fn the_suffix_is_drawn_again_until_the_names_are_free() {
    let folder = Folder::new();
    for rel in [RAW, "notes/202606190001-topic-a.md", "notes/202606190001-topic-b.md"] {
        folder.file("raw", rel);
    }

    let got = plan(&folder, &package(RAW, None, &[]), None, &[]);

    assert_eq!(got, Ok(landing("notes/202606190001-topic-c.md", None)));
}

#[test]
fn a_digest_with_a_name_of_its_own_gets_the_suffix_too() {
    let folder = Folder::new();
    folder.file("digest", "notes/summary.md");

    let got = plan(&folder, &package(RAW, Some("notes/summary.md"), &[]), None, &[]);

    assert_eq!(got, Ok(landing("notes/202606190001-topic-a.md", Some("notes/summary-a.md"))));
}

#[test]
fn a_package_without_a_digest_does_not_care_about_the_digest_layer() {
    let folder = Folder::new();
    folder.file("digest", RAW);

    let got = plan(&folder, &package(RAW, None, &[]), None, &[]);

    assert_eq!(got, Ok(landing(RAW, None)));
}

#[test]
fn the_files_of_the_document_being_replaced_are_not_in_the_way() {
    let folder = Folder::new();
    folder.file("raw", RAW);
    folder.file("raw", EN);
    folder.file("digest", RAW);
    let existing = row(1, RAW, None, &["en"]); // its digest is at the body path
    let others = [existing.clone(), row(2, "notes/another.md", None, &[])];

    let got = plan(&folder, &package(RAW, Some(RAW), &["en"]), Some(&existing), &others);

    assert_eq!(got, Ok(landing(RAW, Some(RAW))));
}

#[test]
fn the_files_of_the_document_being_replaced_are_the_ones_its_row_names() {
    // The row names `old.md`; the package wants `RAW`, where some other file lies.
    let folder = Folder::new();
    folder.file("raw", "notes/old.md");
    folder.file("raw", RAW);
    let existing = row(1, "notes/old.md", None, &[]);

    let got = plan(&folder, &package(RAW, None, &[]), Some(&existing), &[]);

    assert_eq!(got, Ok(landing("notes/202606190001-topic-a.md", None)));
}

#[test]
fn a_file_of_another_document_is_in_the_way_even_when_the_document_being_replaced_has_a_row() {
    let folder = Folder::new();
    folder.file("raw", RAW);
    let existing = row(1, "notes/old.md", None, &[]);
    let others = [row(2, RAW, None, &[])];

    let got = plan(&folder, &package(RAW, None, &[]), Some(&existing), &others);

    assert_eq!(got, Ok(landing("notes/202606190001-topic-a.md", None)));
}

#[test]
fn another_row_that_names_a_path_of_the_document_being_replaced_keeps_it_taken() {
    // The rows are inconsistent (two rows, one path), but the other row's claim stands.
    let folder = Folder::new();
    folder.file("raw", RAW);
    let existing = row(1, RAW, None, &[]);
    let others = [row(2, RAW, None, &[])];

    let got = plan(&folder, &package(RAW, None, &[]), Some(&existing), &others);

    assert_eq!(got, Ok(landing("notes/202606190001-topic-a.md", None)));
}

#[test]
fn a_row_of_the_same_document_among_the_others_does_not_count() {
    let folder = Folder::new();

    let got = plan(&folder, &package(RAW, None, &[]), None, &[row(1, RAW, None, &[])]);

    assert_eq!(got, Ok(landing(RAW, None)));
}

#[test]
fn a_body_that_cannot_take_a_suffix_is_an_error() {
    let folder = Folder::new();
    folder.file("raw", "notes/readme.txt");

    let got = plan(&folder, &package("notes/readme.txt", None, &[]), None, &[]);

    assert!(got.expect_err("no suffix can go before .md").contains("cannot rename"));
}

#[test]
fn a_workspace_folder_that_is_not_there_is_an_error() {
    let folder = Folder::new();
    fs::remove_dir_all(&folder.root).expect("remove");

    assert!(plan(&folder, &package(RAW, None, &[]), None, &[]).is_err());
}
