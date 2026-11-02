use std::fs;
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::*;
use crate::services::doc_zip::tests::{
    body, body_only, forge, full_doc, manifest_bytes, Home, DOC, OTHER_DOC, PROJECT, RAW_PATH,
};

/// A package of the plain document at `RAW_PATH` (body only) plus these entries.
fn plain(extra: &[(&str, &[u8])]) -> Vec<u8> {
    let manifest = manifest_bytes(DOC, RAW_PATH, None, &[]);
    let raw_name = format!("raw/{RAW_PATH}");
    let mut entries: Vec<(&str, &[u8])> = vec![("manifest.json", &manifest), (&raw_name, b"body")];
    entries.extend_from_slice(extra);
    forge(&entries, CompressionMethod::Deflated)
}

/// Unpack over a full local document; the package must be refused and nothing may change.
fn refused(package: &[u8], fragment: &str) {
    let home = Home::new();
    let row = full_doc(&home, "local");
    let before = home.tree();
    let err = unpack_doc(&home.root, PROJECT, DOC, package, Some(&row)).expect_err("must be refused");
    assert!(err.contains(fragment), "wanted {fragment:?} in: {err}");
    assert_eq!(home.tree(), before, "the local files must not change");
}

#[test]
fn entries_that_climb_out_of_the_document_folder_are_refused() {
    for (name, fragment) in [
        ("raw/../../evil.md", "unsafe path"),
        ("raw/notes/../../../evil.md", "unsafe path"),
        ("digest/../evil.md", "unsafe path"),
        ("raw//evil.md", "unsafe path"),
        ("raw/a\\b.md", "unsafe path"),
        ("/etc/evil", "unexpected entry"),
        ("raw\\evil.md", "unexpected entry"),
        ("evil.md", "unexpected entry"),
        ("other/evil.md", "unexpected entry"),
        ("attached/sub/x.png", "unsafe attachment"),
        ("attached/../x.png", "unsafe attachment"),
    ] {
        let package = plain(&[(name, b"evil")]);
        refused(&package, fragment);
    }
}

#[test]
fn a_manifest_path_that_climbs_out_is_refused() {
    for raw in ["../evil.md", "/etc/evil.md", "notes\\evil.md"] {
        let manifest = manifest_bytes(DOC, raw, None, &[]);
        let package = forge(
            &[("manifest.json", &manifest), (&format!("raw/{RAW_PATH}"), b"body")],
            CompressionMethod::Deflated,
        );
        refused(&package, "unsafe path");
    }
    let manifest = manifest_bytes(DOC, RAW_PATH, Some("../evil.md"), &[]);
    let package = forge(
        &[("manifest.json", &manifest), (&format!("raw/{RAW_PATH}"), b"body")],
        CompressionMethod::Deflated,
    );
    refused(&package, "unsafe path");
}

#[test]
fn folders_and_links_are_refused() {
    let manifest = manifest_bytes(DOC, RAW_PATH, None, &[]);
    let options = SimpleFileOptions::default();

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("manifest.json", options).expect("start");
    zip.write_all(&manifest).expect("write");
    zip.add_directory("raw/", options).expect("folder");
    refused(&zip.finish().expect("finish").into_inner(), "not a plain file");

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("manifest.json", options).expect("start");
    zip.write_all(&manifest).expect("write");
    zip.add_symlink(format!("raw/{RAW_PATH}"), "/etc/passwd", options).expect("link");
    refused(&zip.finish().expect("finish").into_inner(), "not a plain file");
}

#[test]
fn a_package_that_uses_a_name_twice_is_refused() {
    // The zip writer refuses a repeated name, so write two names one letter apart and rename.
    let manifest = manifest_bytes(DOC, "notes/aaaa.md", None, &[]);
    let package = forge(
        &[
            ("manifest.json", &manifest),
            ("raw/notes/aaaa.md", b"one"),
            ("raw/notes/aaab.md", b"two"),
        ],
        CompressionMethod::Deflated,
    );
    refused(&rename(&package, b"raw/notes/aaab.md", b"raw/notes/aaaa.md"), "twice");
}

/// `bytes` with every `from` replaced by `to`, which has the same length.
fn rename(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let mut out = bytes.to_vec();
    let mut at = 0;
    while at + from.len() <= out.len() {
        if &out[at..at + from.len()] == from {
            out[at..at + from.len()].copy_from_slice(to);
            at += from.len();
        } else {
            at += 1;
        }
    }
    out
}

#[test]
fn a_manifest_that_does_not_match_the_package_is_refused() {
    let raw = format!("raw/{RAW_PATH}");
    let en = "raw/notes/202606191430-topic-en.md";
    let deflated = CompressionMethod::Deflated;

    // The manifest names another document than the object does.
    let other = manifest_bytes(OTHER_DOC, RAW_PATH, None, &[]);
    refused(&forge(&[("manifest.json", &other), (&raw, b"body")], deflated), "does not match");

    // Listed but absent: a translation, a digest.
    let listed_en = manifest_bytes(DOC, RAW_PATH, None, &["en"]);
    let package = forge(&[("manifest.json", &listed_en), (&raw, b"body")], deflated);
    refused(&package, "lists raw/notes/202606191430-topic-en.md");
    let listed_digest = manifest_bytes(DOC, RAW_PATH, Some(RAW_PATH), &[]);
    let package = forge(&[("manifest.json", &listed_digest), (&raw, b"body")], deflated);
    refused(&package, "lists digest/");
    let listed_body = manifest_bytes(DOC, RAW_PATH, None, &[]);
    refused(&forge(&[("manifest.json", &listed_body)], deflated), "lists raw/");

    // Present but not listed: a translation, a digest.
    refused(&plain(&[(en, b"extra")]), "does not list");
    refused(&plain(&[("digest/notes/x.md", b"extra")]), "does not list");
}

#[test]
fn a_package_that_is_not_a_package_is_refused() {
    refused(b"this is not a zip", "not a zip package");
    refused(&forge(&[("raw/notes/a.md", b"body")], CompressionMethod::Deflated), "expected 1");
    let manifest = b"{ not json";
    let package = forge(
        &[("manifest.json", manifest), (&format!("raw/{RAW_PATH}"), b"body")],
        CompressionMethod::Deflated,
    );
    refused(&package, "manifest.json");
}

#[test]
fn the_object_name_must_be_a_doc_id_and_the_local_row_must_be_that_document() {
    let home = Home::new();
    let package = plain(&[]);
    let err = unpack_doc(&home.root, PROJECT, "short", &package, None).expect_err("bad name");
    assert!(err.contains("invalid doc id"), "{err}");

    let err = unpack_doc(&home.root, PROJECT, OTHER_DOC, &package, None).expect_err("other doc");
    assert!(err.contains("does not match"), "{err}");

    let row = body_only(&home, "a");
    let err = unpack_doc(&home.root, PROJECT, OTHER_DOC, &package, Some(&row)).expect_err("row");
    assert!(err.contains("is not document"), "{err}");
}

#[test]
fn a_package_that_unzips_past_the_limit_is_refused() {
    let home = Home::new();
    let package = plain(&[("attached/big.bin", &[7u8; 200])]);
    let err = unpack_capped(&home.root, PROJECT, DOC, &package, None, 100).expect_err("over limit");
    assert!(err.contains("limit"), "{err}");
    assert!(home.files().is_empty());

    let allowed = unpack_capped(&home.root, PROJECT, DOC, &package, None, 10_000);
    assert!(allowed.is_ok(), "{allowed:?}");
}

/// `package` with the size that entry `name` declares for itself, in both of its headers, set
/// to `declared`. The data is untouched.
fn lie_about_size(package: &[u8], name: &str, declared: u32) -> Vec<u8> {
    let mut out = package.to_vec();
    let central = (0..out.len() - 46)
        .find(|&at| {
            out[at..at + 4] == *b"PK\x01\x02" && {
                let len = u16::from_le_bytes([out[at + 28], out[at + 29]]) as usize;
                out[at + 46..at + 46 + len] == *name.as_bytes()
            }
        })
        .expect("central header");
    let local = u32::from_le_bytes(out[central + 42..central + 46].try_into().expect("offset")) as usize;
    out[central + 24..central + 28].copy_from_slice(&declared.to_le_bytes());
    out[local + 22..local + 26].copy_from_slice(&declared.to_le_bytes());
    out
}

#[test]
fn a_package_that_declares_less_than_it_unzips_to_is_refused() {
    // The limit counts the declared sizes, so the reader must not unzip past a declared size.
    let package = plain(&[("attached/big.bin", &[7u8; 5000])]);
    let lying = lie_about_size(&package, "attached/big.bin", 10);

    let home = Home::new();
    let err = unpack_capped(&home.root, PROJECT, DOC, &lying, None, 1000).expect_err("lying size");
    assert!(err.contains("attached/big.bin"), "{err}");
    assert!(home.files().is_empty());
}

#[test]
fn a_package_that_breaks_while_unzipping_changes_nothing() {
    let marker = b"a body that is long enough to find";
    let manifest = manifest_bytes(DOC, RAW_PATH, None, &[]);
    let mut package = forge(
        &[("manifest.json", &manifest), (&format!("raw/{RAW_PATH}"), marker)],
        CompressionMethod::Stored,
    );
    let at = package.windows(marker.len()).position(|w| w == marker).expect("marker in package");
    package[at] ^= 0xff;
    refused(&package, "raw/notes/202606191430-topic.md");
}

#[test]
fn a_swap_that_fails_half_way_puts_every_old_file_back() {
    // The new body wants the folder `blocked/`, but a plain file of that name is in the way.
    let home = Home::new();
    let row = full_doc(&home, "local");
    home.put("raw", PROJECT, "blocked", b"in the way");
    let before = home.tree();

    let manifest = manifest_bytes(DOC, "blocked/x.md", None, &[]);
    let package = forge(
        &[("manifest.json", &manifest), ("raw/blocked/x.md", b"new"), ("attached/a.png", b"png")],
        CompressionMethod::Deflated,
    );
    let err = unpack_doc(&home.root, PROJECT, DOC, &package, Some(&row)).expect_err("blocked");
    assert!(err.contains("blocked"), "{err}");
    assert_eq!(home.tree(), before);
}

#[test]
fn a_swap_that_fails_late_removes_what_it_already_placed_and_the_folders_it_made() {
    // The body lands first; the digest then meets a file of another document and the swap stops.
    let home = Home::new();
    let row = full_doc(&home, "local");
    home.put("digest", PROJECT, "deep/er/x.md", b"digest of another document");
    let before = home.tree();

    let manifest = manifest_bytes(DOC, "deep/er/x.md", Some("deep/er/x.md"), &[]);
    let package = forge(
        &[("manifest.json", &manifest), ("raw/deep/er/x.md", b"new"), ("digest/deep/er/x.md", b"new")],
        CompressionMethod::Deflated,
    );
    let err = unpack_doc(&home.root, PROJECT, DOC, &package, Some(&row)).expect_err("taken");
    assert!(err.contains("already used"), "{err}");
    assert_eq!(home.tree(), before);
    assert!(!home.file("raw", PROJECT, "deep").exists());
}

#[test]
fn a_file_of_another_document_is_never_overwritten() {
    let home = Home::new();
    home.put("raw", PROJECT, RAW_PATH, b"another document's body");
    let before = home.tree();

    let err = unpack_doc(&home.root, PROJECT, DOC, &plain(&[]), None).expect_err("taken");
    assert!(err.contains("already used"), "{err}");
    assert_eq!(home.tree(), before);
}

#[test]
fn an_attachment_folder_of_the_same_document_that_has_no_row_is_replaced() {
    let home = Home::new();
    home.put_attached(PROJECT, RAW_PATH, DOC, "old.png", b"old");
    let package = plain(&[("attached/new.png", b"new")]);

    unpack_doc(&home.root, PROJECT, DOC, &package, None).expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, RAW_PATH, b"body");
    expected.put_attached(PROJECT, RAW_PATH, DOC, "new.png", b"new");
    assert_eq!(home.files(), expected.files());
}

#[test]
fn a_staging_folder_left_by_a_crash_does_not_get_in_the_way() {
    let home = Home::new();
    home.put(".zip-staging", DOC, &format!("new/raw/{RAW_PATH}/stale.md"), b"stale");
    home.put(".zip-staging", DOC, "old/0/stale.md", b"stale");

    unpack_doc(&home.root, PROJECT, DOC, &plain(&[]), None).expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, RAW_PATH, b"body");
    assert_eq!(home.files(), expected.files());
    assert!(fs::read_dir(&home.root).expect("ws").all(|e| e.expect("entry").file_name() != ".zip-staging"));
}

// --- peek_package ---

#[test]
fn peeking_tells_what_unpacking_would_say_and_touches_nothing() {
    for package_of in [full_doc as fn(&Home, &str) -> DocRow, body_only] {
        let source = Home::new();
        let row = package_of(&source, "a");
        let package = crate::services::doc_zip::pack_doc(&source.root, &row).expect("pack");

        let peeked = peek_package(&package, DOC).expect("peek");

        let target = Home::new();
        let before = target.tree();
        assert_eq!(unpack_doc(&target.root, PROJECT, DOC, &package, None), Ok(peeked));
        assert_ne!(target.tree(), before);
    }
}

#[test]
fn peeking_refuses_what_unpacking_refuses_before_it_touches_the_disk() {
    let raw = format!("raw/{RAW_PATH}");
    let deflated = CompressionMethod::Deflated;
    let other = manifest_bytes(OTHER_DOC, RAW_PATH, None, &[]);
    let listed_en = manifest_bytes(DOC, RAW_PATH, None, &["en"]);
    let climbing = manifest_bytes(DOC, "../evil.md", None, &[]);
    let refused: Vec<(&str, Vec<u8>, &str)> = vec![
        ("not a zip", b"this is not a zip".to_vec(), "not a zip package"),
        ("another document", forge(&[("manifest.json", &other), (&raw, b"b")], deflated), "does not match"),
        ("a listed file is missing", forge(&[("manifest.json", &listed_en), (&raw, b"b")], deflated), "lists raw/"),
        ("an unlisted file", plain(&[("digest/notes/x.md", b"extra")]), "does not list"),
        ("a path that climbs out", forge(&[("manifest.json", &climbing), (&raw, b"b")], deflated), "unsafe path"),
        ("an entry of another kind", plain(&[("other/evil.md", b"evil")]), "unexpected entry"),
    ];
    for (what, package, fragment) in refused {
        let err = peek_package(&package, DOC).expect_err(what);
        assert!(err.contains(fragment), "{what}: wanted {fragment:?} in: {err}");
    }
    let err = peek_package(&plain(&[]), "short").expect_err("bad name");
    assert!(err.contains("invalid doc id"), "{err}");
}

// --- unpack_doc_to ---

const RENAMED: &str = "notes/202606191430-topic-sfx.md";
const RENAMED_EN: &str = "notes/202606191430-topic-sfx-en.md";
const RENAMED_ZH: &str = "notes/202606191430-topic-sfx-zh.md";

fn landing(raw: &str, digest: Option<&str>) -> Landing {
    Landing { raw_path: raw.to_string(), digest_path: digest.map(str::to_string) }
}

fn full_package(tag: &str) -> (Home, Vec<u8>) {
    let source = Home::new();
    let row = full_doc(&source, tag);
    let package = crate::services::doc_zip::pack_doc(&source.root, &row).expect("pack");
    (source, package)
}

#[test]
fn a_document_lands_under_other_names_and_everything_follows_the_body() {
    let (_source, package) = full_package("a");
    let home = Home::new();

    let unpacked =
        unpack_doc_to(&home.root, PROJECT, DOC, &package, None, &landing(RENAMED, Some(RENAMED))).expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, RENAMED, &body("a"));
    expected.put("raw", PROJECT, RENAMED_EN, b"english a");
    expected.put("raw", PROJECT, RENAMED_ZH, "中文 a".as_bytes());
    expected.put("digest", PROJECT, RENAMED, b"# digest a\n");
    expected.put_attached(PROJECT, RENAMED, DOC, "figure.png", &[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 255]);
    expected.put_attached(PROJECT, RENAMED, DOC, "notes.txt", b"note a");
    assert_eq!(home.files(), expected.files());
    assert_eq!(
        unpacked,
        Unpacked {
            raw_path: RENAMED.to_string(),
            digest_path: Some(RENAMED.to_string()),
            translations: vec!["en".to_string(), "zh".to_string()],
        }
    );
}

#[test]
fn a_digest_with_a_name_of_its_own_lands_under_the_name_it_is_given() {
    let manifest = manifest_bytes(DOC, RAW_PATH, Some("notes/summary.md"), &[]);
    let package = forge(
        &[
            ("manifest.json", &manifest),
            (&format!("raw/{RAW_PATH}"), b"body"),
            ("digest/notes/summary.md", b"digest"),
        ],
        CompressionMethod::Deflated,
    );
    let home = Home::new();

    let unpacked = unpack_doc_to(&home.root, PROJECT, DOC, &package, None, &landing(RENAMED, Some("notes/summary-sfx.md")))
        .expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, RENAMED, b"body");
    expected.put("digest", PROJECT, "notes/summary-sfx.md", b"digest");
    assert_eq!(home.files(), expected.files());
    assert_eq!(unpacked.digest_path.as_deref(), Some("notes/summary-sfx.md"));
}

#[test]
fn landing_where_the_manifest_says_is_the_same_as_unpacking() {
    let (_source, package) = full_package("a");
    let plain_home = Home::new();
    let landed_home = Home::new();

    let plain_result = unpack_doc(&plain_home.root, PROJECT, DOC, &package, None).expect("unpack");
    let landed_result =
        unpack_doc_to(&landed_home.root, PROJECT, DOC, &package, None, &landing(RAW_PATH, Some(RAW_PATH))).expect("unpack");

    assert_eq!(plain_home.tree(), landed_home.tree());
    assert_eq!(plain_result, landed_result);
}

#[test]
fn landing_over_a_local_document_removes_all_its_files_and_places_the_package_under_the_new_names() {
    let (_source, package) = full_package("cloud");
    let home = Home::new();
    let row = full_doc(&home, "local");
    home.put_attached(PROJECT, RAW_PATH, DOC, "only-here.png", b"x");

    unpack_doc_to(&home.root, PROJECT, DOC, &package, Some(&row), &landing(RENAMED, Some(RENAMED))).expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, RENAMED, &body("cloud"));
    expected.put("raw", PROJECT, RENAMED_EN, b"english cloud");
    expected.put("raw", PROJECT, RENAMED_ZH, "中文 cloud".as_bytes());
    expected.put("digest", PROJECT, RENAMED, b"# digest cloud\n");
    expected.put_attached(PROJECT, RENAMED, DOC, "figure.png", &[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 255]);
    expected.put_attached(PROJECT, RENAMED, DOC, "notes.txt", b"note cloud");
    assert_eq!(home.files(), expected.files());
}

#[test]
fn a_landing_that_does_not_fit_the_package_is_refused_and_nothing_changes() {
    let (_source, with_digest) = full_package("a");
    let without_digest = plain(&[]);
    let cases: Vec<(&str, &[u8], Landing, &str)> = vec![
        ("a digest in the package, none in the landing", &with_digest, landing(RENAMED, None), "disagree"),
        ("no digest in the package, one in the landing", &without_digest, landing(RENAMED, Some(RENAMED)), "disagree"),
        ("a body that climbs out", &without_digest, landing("../evil.md", None), "unsafe path"),
        ("a body with a backslash", &without_digest, landing("notes\\evil.md", None), "unsafe path"),
        ("a digest that climbs out", &with_digest, landing(RENAMED, Some("../evil.md")), "unsafe path"),
        ("an empty body path", &without_digest, landing("", None), "unsafe path"),
        ("translations and a body that is not .md", &with_digest, landing("notes/body.txt", Some("notes/body.txt")), "must end in .md"),
    ];
    for (what, package, landing, fragment) in cases {
        let home = Home::new();
        let row = full_doc(&home, "local");
        let before = home.tree();

        let err = unpack_doc_to(&home.root, PROJECT, DOC, package, Some(&row), &landing).expect_err(what);

        assert!(err.contains(fragment), "{what}: wanted {fragment:?} in: {err}");
        assert_eq!(home.tree(), before, "{what}");
    }
}

#[test]
fn a_file_of_another_document_is_never_overwritten_at_the_landing_either() {
    let home = Home::new();
    home.put("raw", PROJECT, RENAMED, b"another document's body");
    let before = home.tree();

    let err = unpack_doc_to(&home.root, PROJECT, DOC, &plain(&[]), None, &landing(RENAMED, None)).expect_err("taken");

    assert!(err.contains("already used"), "{err}");
    assert_eq!(home.tree(), before);
}

#[test]
fn a_package_that_declares_more_than_the_limit_is_refused_by_peek_and_by_unpack_but_a_smaller_one_is_fine() {
    let package = plain(&[("attached/big.bin", b"x")]);
    let over = lie_about_size(&package, "attached/big.bin", (MAX_UNPACKED_BYTES + 1) as u32);
    let under = lie_about_size(&package, "attached/big.bin", 50 * 1024 * 1024);

    let err = peek_package(&over, DOC).expect_err("over the limit");
    assert!(err.contains("limit"), "{err}");
    let home = Home::new();
    let err = unpack_doc(&home.root, PROJECT, DOC, &over, None).expect_err("over the limit");
    assert!(err.contains("limit"), "{err}");
    assert!(home.files().is_empty());

    assert!(peek_package(&under, DOC).is_ok());
}

#[test]
fn the_attachment_folder_lands_next_to_the_body_in_its_new_folder() {
    let (_source, package) = full_package("a");
    let home = Home::new();
    let moved = "archive/2026/202606191430-topic-sfx.md";

    unpack_doc_to(&home.root, PROJECT, DOC, &package, None, &landing(moved, Some(moved))).expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, moved, &body("a"));
    expected.put("raw", PROJECT, "archive/2026/202606191430-topic-sfx-en.md", b"english a");
    expected.put("raw", PROJECT, "archive/2026/202606191430-topic-sfx-zh.md", "中文 a".as_bytes());
    expected.put("digest", PROJECT, moved, b"# digest a\n");
    expected.put_attached(PROJECT, moved, DOC, "figure.png", &[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 255]);
    expected.put_attached(PROJECT, moved, DOC, "notes.txt", b"note a");
    assert_eq!(home.files(), expected.files());
}
