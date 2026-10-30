//! Document-owned companion files: `<md dir>/.attached/{doc_id}/{basename}`.
//! The directory is the index; the database has no attachment column.
//! Read still joins the href against the Markdown directory and refuses `..`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::spark_data::is_valid_doc_id;

pub(crate) const ATTACHED_DIR_NAME: &str = ".attached";

fn invalid_path() -> Value {
    json!({ "error": "Invalid path", "_status": 400 })
}

fn validate_doc_id(doc_id: &str) -> Result<&str, Value> {
    if !is_valid_doc_id(doc_id) {
        return Err(json!({ "error": "Invalid id", "_status": 400 }));
    }
    Ok(doc_id)
}

fn validate_basename(basename: &str) -> Result<&str, Value> {
    let name = basename.trim();
    if name.is_empty()
        || name == "."
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
    {
        return Err(invalid_path());
    }
    Ok(name)
}

/// `<md_dir>/.attached/{doc_id}`.
pub(crate) fn attached_dir(md_dir: &Path, doc_id: &str) -> Result<PathBuf, Value> {
    let doc_id = validate_doc_id(doc_id)?;
    Ok(md_dir.join(ATTACHED_DIR_NAME).join(doc_id))
}

/// `<md_dir>/.attached/{doc_id}/{basename}`.
pub(crate) fn attached_file(
    md_dir: &Path,
    doc_id: &str,
    basename: &str,
) -> Result<PathBuf, Value> {
    let basename = validate_basename(basename)?;
    Ok(attached_dir(md_dir, doc_id)?.join(basename))
}

/// Relative href from the Markdown file: `.attached/{doc_id}/{basename}`.
pub(crate) fn attached_rel(doc_id: &str, basename: &str) -> Result<String, Value> {
    let doc_id = validate_doc_id(doc_id)?;
    let basename = validate_basename(basename)?;
    Ok(format!("{ATTACHED_DIR_NAME}/{doc_id}/{basename}"))
}

/// Copy `<src_md_dir>/.attached/{doc_id}` onto the same layout under `dest_md_dir`.
/// Missing source is a no-op. An existing dest directory is 409. A failed copy
/// removes the dest directory.
pub(crate) fn copy_attached_dir(
    src_md_dir: &Path,
    dest_md_dir: &Path,
    doc_id: &str,
) -> Result<Option<PathBuf>, Value> {
    let src = attached_dir(src_md_dir, doc_id)?;
    if !src.exists() {
        return Ok(None);
    }
    if !src.is_dir() {
        return Err(invalid_path());
    }
    let dest = attached_dir(dest_md_dir, doc_id)?;
    if dest.exists() {
        return Err(json!({ "error": "File already exists", "_status": 409 }));
    }
    if let Err(e) = copy_tree(&src, &dest) {
        let _ = fs::remove_dir_all(&dest);
        return Err(json!({ "error": e, "_status": 500 }));
    }
    Ok(Some(dest))
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_str().ok_or_else(|| "Invalid path".to_string())?;
        if name_str.is_empty()
            || name_str == "."
            || name_str.contains("..")
            || name_str.contains('/')
            || name_str.contains('\\')
        {
            return Err("Invalid path".into());
        }
        let from = entry.path();
        let to = dest.join(name);
        if from.is_dir() {
            copy_tree(&from, &to)?;
        } else if to.exists() {
            return Err("File already exists".into());
        } else {
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Remove `<md_dir>/.attached/{doc_id}` when it exists. Missing is a no-op.
/// An empty `.attached` parent is dropped.
pub(crate) fn remove_attached_dir(md_dir: &Path, doc_id: &str) -> Result<(), Value> {
    let dest = attached_dir(md_dir, doc_id)?;
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::remove_dir(parent);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_attached.rs"]
mod tests;
