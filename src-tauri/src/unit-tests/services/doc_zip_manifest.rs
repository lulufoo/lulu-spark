use serde_json::{json, Value};

use super::*;
use crate::services::doc_zip::tests::{DOC, OTHER_DOC, RAW_PATH};

fn manifest(digest: Option<&str>, langs: &[&str]) -> Manifest {
    Manifest::new(DOC, RAW_PATH, digest, langs.iter().map(|l| l.to_string()).collect())
}

fn parse(value: Value) -> Result<Manifest, String> {
    Manifest::parse(value.to_string().as_bytes(), DOC)
}

#[test]
fn a_manifest_reads_back_what_was_written() {
    let written = manifest(Some(RAW_PATH), &["en", "zh"]);
    let read = Manifest::parse(&written.to_bytes().expect("bytes"), DOC).expect("parse");
    assert_eq!(read, written);

    let bare = manifest(None, &[]);
    assert_eq!(Manifest::parse(&bare.to_bytes().expect("bytes"), DOC).expect("parse"), bare);
}

#[test]
fn a_manifest_has_five_fields_and_no_version_number() {
    let bytes = manifest(None, &[]).to_bytes().expect("bytes");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    let mut keys: Vec<&str> = value.as_object().expect("object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["digest_path", "doc_id", "format", "raw_path", "translations"]);
    assert_eq!(value["digest_path"], Value::Null);
}

#[test]
fn raw_rels_lists_the_body_then_each_translation() {
    let rels = manifest(None, &["en", "zh"]).raw_rels();
    assert_eq!(
        rels,
        ["notes/202606191430-topic.md", "notes/202606191430-topic-en.md", "notes/202606191430-topic-zh.md"]
    );
}

#[test]
fn a_manifest_for_another_document_is_refused() {
    let other = Manifest::new(OTHER_DOC, RAW_PATH, None, Vec::new());
    let err = Manifest::parse(&other.to_bytes().expect("bytes"), DOC).expect_err("other doc");
    assert!(err.contains("does not match"), "{err}");

    let err = Manifest::new("not-an-id", RAW_PATH, None, Vec::new())
        .check("not-an-id")
        .expect_err("bad id");
    assert!(err.contains("does not match"), "{err}");
}

#[test]
fn a_manifest_of_another_format_or_with_extra_fields_is_refused() {
    let good = json!({
        "format": 1, "doc_id": DOC, "raw_path": RAW_PATH, "digest_path": null, "translations": []
    });
    assert!(parse(good.clone()).is_ok());

    let mut newer = good.clone();
    newer["format"] = json!(2);
    assert!(parse(newer).expect_err("format 2").contains("unsupported format"));

    let mut extra = good.clone();
    extra["version"] = json!(3);
    assert!(parse(extra).is_err());

    assert!(Manifest::parse(b"{not json", DOC).is_err());
}

#[test]
fn paths_that_could_leave_the_document_folder_are_refused() {
    for bad in [
        "../evil.md",
        "notes/../../evil.md",
        "/etc/passwd",
        "notes//a.md",
        "notes/",
        "./a.md",
        "notes/./a.md",
        "notes\\a.md",
        "a\0.md",
        "",
    ] {
        assert!(check_member_path(bad).is_err(), "{bad:?} must be refused");
        let manifest = Manifest::new(DOC, bad, None, Vec::new());
        assert!(manifest.check(DOC).is_err(), "raw_path {bad:?} must be refused");
        let manifest = Manifest::new(DOC, RAW_PATH, Some(bad), Vec::new());
        assert!(manifest.check(DOC).is_err(), "digest_path {bad:?} must be refused");
    }
    for good in ["a.md", "notes/a.md", "a/b/c/202606191430-x.md", "文档/中文.md"] {
        assert!(check_member_path(good).is_ok(), "{good:?} must be accepted");
    }
}

#[test]
fn attachment_names_follow_the_attached_folder_rule() {
    for bad in ["", " ", ".", "..", "a/b.png", "sub/", "a\\b.png", " a.png", "a.png ", "a..png", "a\0b"] {
        assert!(check_attached_name(bad).is_err(), "{bad:?} must be refused");
    }
    for good in ["figure.png", "图 1.png", ".hidden", "a.b.c"] {
        assert!(check_attached_name(good).is_ok(), "{good:?} must be accepted");
    }
}

#[test]
fn translation_languages_must_be_two_lowercase_letters_and_unique() {
    for bad in [vec!["english"], vec!["EN"], vec!["e"], vec!["en", "en"], vec![""], vec!["e1"]] {
        let err = manifest(None, &bad).check(DOC).expect_err("bad languages");
        assert!(err.contains("translation language"), "{bad:?}: {err}");
    }
}

#[test]
fn a_body_that_is_not_markdown_cannot_have_translations() {
    let manifest = Manifest::new(DOC, "notes/a.txt", None, vec!["en".to_string()]);
    assert!(manifest.check(DOC).expect_err("not md").contains("must end in .md"));
    assert!(Manifest::new(DOC, "notes/a.txt", None, Vec::new()).check(DOC).is_ok());
}
