use std::fs;

use serde_json::{json, Value};

use super::*;
use crate::services::doc_zip::tests::{
    body, body_only, entry_bytes, full_doc, names, Home, DOC, EN_PATH, PROJECT, RAW_PATH, ZH_PATH,
};

#[test]
fn a_full_document_packs_into_the_agreed_layout() {
    let home = Home::new();
    let row = full_doc(&home, "a");
    let package = pack_doc(&home.root, &row).expect("pack");

    assert_eq!(
        names(&package),
        [
            "manifest.json",
            &format!("raw/{RAW_PATH}"),
            &format!("raw/{EN_PATH}"),
            &format!("raw/{ZH_PATH}"),
            &format!("digest/{RAW_PATH}"),
            "attached/figure.png",
            "attached/notes.txt",
        ]
    );
    let manifest: Value = serde_json::from_slice(&entry_bytes(&package, "manifest.json")).expect("json");
    assert_eq!(
        manifest,
        json!({
            "format": 1,
            "doc_id": DOC,
            "raw_path": RAW_PATH,
            "digest_path": RAW_PATH,
            "translations": ["en", "zh"],
        })
    );
}

#[test]
fn the_files_go_in_as_the_exact_bytes_on_disk() {
    let home = Home::new();
    let row = full_doc(&home, "a");
    let package = pack_doc(&home.root, &row).expect("pack");

    assert_eq!(entry_bytes(&package, &format!("raw/{RAW_PATH}")), body("a"));
    assert_eq!(
        entry_bytes(&package, &format!("digest/{RAW_PATH}")),
        fs::read(home.file("digest", PROJECT, RAW_PATH)).expect("digest")
    );
    assert_eq!(
        entry_bytes(&package, "attached/figure.png"),
        fs::read(home.attached(PROJECT, RAW_PATH, DOC, "figure.png")).expect("attachment")
    );
}

#[test]
fn a_document_with_only_a_body_is_packed_the_same_way() {
    let home = Home::new();
    let row = body_only(&home, "a");
    let package = pack_doc(&home.root, &row).expect("pack");

    assert_eq!(names(&package), ["manifest.json", &format!("raw/{RAW_PATH}")]);
    let manifest: Value = serde_json::from_slice(&entry_bytes(&package, "manifest.json")).expect("json");
    assert_eq!(manifest["digest_path"], Value::Null);
    assert_eq!(manifest["translations"], json!([]));
}

#[test]
fn an_empty_attachment_folder_adds_nothing() {
    let home = Home::new();
    let row = body_only(&home, "a");
    let md_dir = home.file("raw", PROJECT, RAW_PATH).parent().expect("md dir").to_path_buf();
    fs::create_dir_all(md_dir.join(".attached").join(DOC)).expect("empty folder");

    let package = pack_doc(&home.root, &row).expect("pack");
    assert_eq!(names(&package), ["manifest.json", &format!("raw/{RAW_PATH}")]);
}

#[test]
fn the_same_document_packs_to_the_same_bytes() {
    let home = Home::new();
    let row = full_doc(&home, "a");
    let first = pack_doc(&home.root, &row).expect("first");
    let second = pack_doc(&home.root, &row).expect("second");
    assert_eq!(first, second);
}

#[test]
fn a_file_the_row_names_but_the_disk_lacks_is_an_error() {
    for (layer, rel) in [("raw", RAW_PATH), ("raw", EN_PATH), ("digest", RAW_PATH)] {
        let home = Home::new();
        let row = full_doc(&home, "a");
        fs::remove_file(home.file(layer, PROJECT, rel)).expect("remove");
        let err = pack_doc(&home.root, &row).expect_err("missing file");
        assert!(err.contains(rel), "{layer}/{rel}: {err}");
    }
}

#[test]
fn an_attachment_folder_with_a_subfolder_is_an_error() {
    let home = Home::new();
    let row = full_doc(&home, "a");
    let sub = home.attached(PROJECT, RAW_PATH, DOC, "sub");
    fs::create_dir_all(&sub).expect("subfolder");

    let err = pack_doc(&home.root, &row).expect_err("subfolder");
    assert!(err.contains("not a plain file"), "{err}");
}

#[test]
fn an_attachment_that_unpack_would_refuse_is_an_error_at_pack_time() {
    let home = Home::new();
    let row = body_only(&home, "a");
    home.put_attached(PROJECT, RAW_PATH, DOC, "a..b.png", b"x");

    let err = pack_doc(&home.root, &row).expect_err("unsafe name");
    assert!(err.contains("unsafe attachment name"), "{err}");
}

#[test]
fn a_row_that_unpack_would_refuse_is_an_error_at_pack_time() {
    let home = Home::new();

    // The file exists on disk, but the language is not two lowercase letters.
    let mut row = full_doc(&home, "a");
    home.put("raw", PROJECT, "notes/202606191430-topic-EN.md", b"english");
    row.translations = json!(["EN"]);
    let err = pack_doc(&home.root, &row).expect_err("bad language");
    assert!(err.contains("translation language"), "{err}");

    // A body that is not Markdown has no translation path.
    let mut row = body_only(&home, "a");
    row.translations = json!(["en"]);
    row.raw_path = "notes/a.txt".to_string();
    home.put("raw", PROJECT, "notes/a.txt", b"text");
    let err = pack_doc(&home.root, &row).expect_err("not markdown");
    assert!(err.contains("must end in .md"), "{err}");

    let mut row = full_doc(&home, "a");
    row.raw_path = "../escape.md".to_string();
    assert!(pack_doc(&home.root, &row).is_err());

    let mut row = body_only(&home, "a");
    row.doc_id = "short".to_string();
    assert!(pack_doc(&home.root, &row).is_err());
}

#[test]
fn a_row_with_no_translation_list_packs_as_having_none() {
    let home = Home::new();
    let mut row = body_only(&home, "a");
    row.translations = Value::Null;
    let package = pack_doc(&home.root, &row).expect("pack");
    let manifest: Value = serde_json::from_slice(&entry_bytes(&package, "manifest.json")).expect("json");
    assert_eq!(manifest["translations"], json!([]));
}
