//! Fixtures shared by the package tests, and the round trip: pack a document, unpack it elsewhere.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::json;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::{pack_doc, unpack_doc, Unpacked};
use crate::services::spark_data::DocRow;

pub(super) const DOC: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const OTHER_DOC: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const PROJECT: &str = "proj1";
pub(super) const RAW_PATH: &str = "notes/202606191430-topic.md";
pub(super) const EN_PATH: &str = "notes/202606191430-topic-en.md";
pub(super) const ZH_PATH: &str = "notes/202606191430-topic-zh.md";

/// A temp folder with one workspace folder `ws` in it. Snapshots cover the whole temp folder, so
/// a file written outside `ws` shows up too.
pub(super) struct Home {
    pub(super) tmp: TempDir,
    pub(super) root: PathBuf,
}

impl Home {
    pub(super) fn new() -> Self {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path().join("ws");
        fs::create_dir_all(&root).expect("workspace folder");
        Self { tmp, root }
    }

    pub(super) fn file(&self, layer: &str, project: &str, rel: &str) -> PathBuf {
        self.root.join(layer).join(project).join(rel)
    }

    pub(super) fn put(&self, layer: &str, project: &str, rel: &str, bytes: &[u8]) {
        write_file(&self.file(layer, project, rel), bytes);
    }

    /// `<folder of raw_rel>/.attached/{doc_id}/{name}` in the raw layer.
    pub(super) fn put_attached(&self, project: &str, raw_rel: &str, doc_id: &str, name: &str, bytes: &[u8]) {
        let raw = self.file("raw", project, raw_rel);
        let dir = raw.parent().expect("md dir").join(".attached").join(doc_id);
        write_file(&dir.join(name), bytes);
    }

    pub(super) fn attached(&self, project: &str, raw_rel: &str, doc_id: &str, name: &str) -> PathBuf {
        let raw = self.file("raw", project, raw_rel);
        raw.parent().expect("md dir").join(".attached").join(doc_id).join(name)
    }

    /// Every file and every folder under the temp folder; a folder maps to `None`.
    pub(super) fn tree(&self) -> BTreeMap<String, Option<Vec<u8>>> {
        let mut out = BTreeMap::new();
        walk(self.tmp.path(), self.tmp.path(), &mut out);
        out
    }

    /// Like [`Home::tree`] without the folders.
    pub(super) fn files(&self) -> BTreeMap<String, Vec<u8>> {
        self.tree().into_iter().filter_map(|(path, bytes)| bytes.map(|b| (path, b))).collect()
    }
}

fn write_file(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("parent folder");
    fs::write(path, bytes).expect("write");
}

fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
    for entry in fs::read_dir(dir).expect("read_dir") {
        let path = entry.expect("entry").path();
        let rel = path.strip_prefix(base).expect("under base").to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            out.insert(rel, None);
            walk(base, &path, out);
        } else {
            out.insert(rel, Some(fs::read(&path).expect("read")));
        }
    }
}

/// A body that is not valid UTF-8 and holds a NUL: the package must carry bytes, not text.
pub(super) fn body(tag: &str) -> Vec<u8> {
    let mut bytes = format!("# Topic {tag}\n\nbody\r\n").into_bytes();
    bytes.extend_from_slice(&[0xff, 0xfe, 0x00, b'\n']);
    bytes
}

/// A document with a digest, translations `en` and `zh` and two attachments. Returns its row.
pub(super) fn full_doc(home: &Home, tag: &str) -> DocRow {
    home.put("raw", PROJECT, RAW_PATH, &body(tag));
    home.put("raw", PROJECT, EN_PATH, format!("english {tag}").as_bytes());
    home.put("raw", PROJECT, ZH_PATH, format!("中文 {tag}").as_bytes());
    home.put("digest", PROJECT, RAW_PATH, format!("# digest {tag}\n").as_bytes());
    home.put_attached(PROJECT, RAW_PATH, DOC, "figure.png", &[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 255]);
    home.put_attached(PROJECT, RAW_PATH, DOC, "notes.txt", format!("note {tag}").as_bytes());
    let mut row = DocRow::new(DOC, RAW_PATH, PROJECT);
    row.digest_path = Some(RAW_PATH.to_string());
    row.translations = json!(["en", "zh"]);
    row
}

/// A document that is only a body. Returns its row.
pub(super) fn body_only(home: &Home, tag: &str) -> DocRow {
    home.put("raw", PROJECT, RAW_PATH, &body(tag));
    DocRow::new(DOC, RAW_PATH, PROJECT)
}

/// A zip with exactly these entries, in this order, under exactly these names.
pub(super) fn forge(entries: &[(&str, &[u8])], method: CompressionMethod) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default().compression_method(method)).expect("start");
        zip.write_all(bytes).expect("write");
    }
    zip.finish().expect("finish").into_inner()
}

pub(super) fn manifest_bytes(doc_id: &str, raw_path: &str, digest: Option<&str>, langs: &[&str]) -> Vec<u8> {
    let manifest = json!({
        "format": 1,
        "doc_id": doc_id,
        "raw_path": raw_path,
        "digest_path": digest,
        "translations": langs,
    });
    serde_json::to_vec(&manifest).expect("manifest json")
}

pub(super) fn names(package: &[u8]) -> Vec<String> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("zip");
    (0..archive.len())
        .map(|i| archive.by_index(i).expect("entry").name().expect("name").into_owned())
        .collect()
}

pub(super) fn entry_bytes(package: &[u8], name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("zip");
    let mut bytes = Vec::new();
    archive.by_name(name).expect("entry").read_to_end(&mut bytes).expect("read");
    bytes
}

#[test]
fn a_full_document_unpacks_elsewhere_byte_for_byte() {
    let source = Home::new();
    let row = full_doc(&source, "a");
    let package = pack_doc(&source.root, &row).expect("pack");

    let target = Home::new();
    let unpacked = unpack_doc(&target.root, PROJECT, DOC, &package, None).expect("unpack");

    assert_eq!(target.tree(), source.tree());
    assert_eq!(
        unpacked,
        Unpacked {
            raw_path: RAW_PATH.to_string(),
            digest_path: Some(RAW_PATH.to_string()),
            translations: vec!["en".to_string(), "zh".to_string()],
        }
    );
}

#[test]
fn a_document_with_only_a_body_makes_the_round_trip_too() {
    let source = Home::new();
    let row = body_only(&source, "a");
    let package = pack_doc(&source.root, &row).expect("pack");

    let target = Home::new();
    let unpacked = unpack_doc(&target.root, PROJECT, DOC, &package, None).expect("unpack");

    assert_eq!(target.tree(), source.tree());
    assert_eq!(
        unpacked,
        Unpacked { raw_path: RAW_PATH.to_string(), digest_path: None, translations: Vec::new() }
    );
}

#[test]
fn unpacking_over_a_local_document_replaces_every_one_of_its_files() {
    let source = Home::new();
    let cloud_row = full_doc(&source, "cloud");
    let package = pack_doc(&source.root, &cloud_row).expect("pack");

    let local = Home::new();
    let mut row = full_doc(&local, "local");
    local.put("raw", PROJECT, "notes/202606191430-topic-fr.md", b"only here");
    local.put_attached(PROJECT, RAW_PATH, DOC, "only-here.png", b"x");
    row.translations = json!(["en", "zh", "fr"]);
    for home in [&source, &local] {
        home.put("raw", PROJECT, "notes/202606191430-other.md", b"another document");
        home.put_attached(PROJECT, RAW_PATH, OTHER_DOC, "keep.png", b"keep");
    }

    unpack_doc(&local.root, PROJECT, DOC, &package, Some(&row)).expect("unpack");

    assert_eq!(local.files(), source.files());
}

#[test]
fn a_package_without_a_digest_removes_the_local_digest() {
    let source = Home::new();
    let cloud_row = body_only(&source, "cloud");
    let package = pack_doc(&source.root, &cloud_row).expect("pack");

    let local = Home::new();
    let row = full_doc(&local, "local");
    unpack_doc(&local.root, PROJECT, DOC, &package, Some(&row)).expect("unpack");

    assert_eq!(local.files(), source.files());
    // The attachment folder is gone, and so is the `.attached` folder that held only it.
    let md_dir = local.file("raw", PROJECT, RAW_PATH).parent().expect("md dir").to_path_buf();
    assert!(!md_dir.join(".attached").exists());
}

#[test]
fn a_digest_at_the_body_path_of_a_row_without_digest_path_is_removed_too() {
    // The readers look for the digest at the body path when the row has no `digest_path`.
    let source = Home::new();
    let cloud_row = body_only(&source, "cloud");
    let package = pack_doc(&source.root, &cloud_row).expect("pack");

    let local = Home::new();
    let row = body_only(&local, "local");
    local.put("digest", PROJECT, RAW_PATH, b"a digest the row does not name");
    unpack_doc(&local.root, PROJECT, DOC, &package, Some(&row)).expect("unpack");

    assert_eq!(local.files(), source.files());
}

#[test]
fn a_package_that_moves_the_body_moves_the_attachment_folder_with_it() {
    let moved = "archive/2026/202607011200-moved.md";
    let package = forge(
        &[
            ("manifest.json", &manifest_bytes(DOC, moved, None, &[])),
            (&format!("raw/{moved}"), b"moved body"),
            ("attached/a.png", b"png"),
        ],
        CompressionMethod::Deflated,
    );

    let local = Home::new();
    let row = full_doc(&local, "local");
    unpack_doc(&local.root, PROJECT, DOC, &package, Some(&row)).expect("unpack");

    let expected = Home::new();
    expected.put("raw", PROJECT, moved, b"moved body");
    expected.put_attached(PROJECT, moved, DOC, "a.png", b"png");
    assert_eq!(local.files(), expected.files());
}
