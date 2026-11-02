//! `pack_doc`: read a document's files and write them as one package.

use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::services::docs::attached::attached_dir;
use crate::services::docs::store::doc_file_path;
use crate::services::spark_data::DocRow;

use super::error_text;
use super::manifest::{
    check_attached_name, Manifest, ATTACHED_DIR, DIGEST_DIR, MANIFEST_NAME, RAW_DIR,
};

/// The package of `doc`: its body, digest, translations and attachments, read as bytes from the
/// workspace folder `workspace_root`. The same document packs to the same bytes.
///
/// A file the row names but the disk lacks is an error, so a broken document is never uploaded
/// as if it were whole. The size of the package is the caller's to check.
pub fn pack_doc(workspace_root: &Path, doc: &DocRow) -> Result<Vec<u8>, String> {
    let translations = doc
        .translations
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    let manifest = Manifest::new(&doc.doc_id, &doc.raw_path, doc.digest_path.as_deref(), translations);
    manifest.check(&doc.doc_id)?;

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    add(&mut zip, MANIFEST_NAME, &manifest.to_bytes()?)?;
    for rel in manifest.raw_rels() {
        let path = doc_file_path(workspace_root, "raw", &doc.project_id, &rel).map_err(error_text)?;
        add(&mut zip, &format!("{RAW_DIR}/{rel}"), &read(&path)?)?;
    }
    if let Some(rel) = &manifest.digest_path {
        let path = doc_file_path(workspace_root, "digest", &doc.project_id, rel).map_err(error_text)?;
        add(&mut zip, &format!("{DIGEST_DIR}/{rel}"), &read(&path)?)?;
    }
    let raw = doc_file_path(workspace_root, "raw", &doc.project_id, &doc.raw_path).map_err(error_text)?;
    if let Some(md_dir) = raw.parent() {
        add_attachments(&mut zip, md_dir, &doc.doc_id)?;
    }
    let cursor = zip.finish().map_err(|e| format!("zip: {e}"))?;
    Ok(cursor.into_inner())
}

/// The files of `<md_dir>/.attached/{doc_id}/` in name order. Only plain files belong there.
fn add_attachments(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    md_dir: &Path,
    doc_id: &str,
) -> Result<(), String> {
    let dir = attached_dir(md_dir, doc_id).map_err(error_text)?;
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|name| format!("{}: attachment name {name:?} is not text", dir.display()))?;
        check_attached_name(&name)?;
        if !entry.path().is_file() {
            return Err(format!("attachment {name:?} of {doc_id} is not a plain file"));
        }
        files.push(name);
    }
    files.sort();
    for name in files {
        add(zip, &format!("{ATTACHED_DIR}/{name}"), &read(&dir.join(&name))?)?;
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn add(zip: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, bytes: &[u8]) -> Result<(), String> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(name, options).map_err(|e| format!("zip {name}: {e}"))?;
    zip.write_all(bytes).map_err(|e| format!("zip {name}: {e}"))
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_zip_pack.rs"]
mod tests;
