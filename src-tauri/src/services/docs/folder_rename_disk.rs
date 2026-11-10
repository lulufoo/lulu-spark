//! File moves for a folder rename. The database stays with the caller.
//! A recorded pair is `(old path, new path)` so a failure can rename it back.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::attached::attached_dir;
use super::resolve::{conflict, internal};
use super::store::doc_file_path;

pub(super) fn move_rel(
    root: &Path,
    layer: &str,
    project: &str,
    from: &str,
    to: &str,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), Value> {
    let src = doc_file_path(root, layer, project, from)?;
    let dest = doc_file_path(root, layer, project, to)?;
    move_path(&src, &dest, files)
}

pub(super) fn move_attached(
    root: &Path,
    project: &str,
    doc_id: &str,
    old_rel: &str,
    new_rel: &str,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), Value> {
    let old_raw = doc_file_path(root, "raw", project, old_rel)?;
    let new_raw = doc_file_path(root, "raw", project, new_rel)?;
    let (Some(old_parent), Some(new_parent)) = (old_raw.parent(), new_raw.parent()) else {
        return Ok(());
    };
    let src = attached_dir(old_parent, doc_id)?;
    if !src.exists() {
        return Ok(());
    }
    move_path(&src, &attached_dir(new_parent, doc_id)?, files)
}

pub(super) fn shift_tree(from: &Path, to: &Path, files: &mut Vec<(PathBuf, PathBuf)>) -> Result<(), String> {
    if !from.exists() {
        return Ok(());
    }
    fs::create_dir_all(to).map_err(|err| err.to_string())?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(from).map_err(|err| err.to_string())? {
        entries.push(entry.map_err(|err| err.to_string())?);
    }
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let src = entry.path();
        let dest = to.join(entry.file_name());
        if src.is_dir() {
            shift_tree(&src, &dest, files)?;
            let _ = fs::remove_dir(&src);
        } else {
            move_path(&src, &dest, files).map_err(|err| {
                err["error"].as_str().unwrap_or("move failed").to_string()
            })?;
        }
    }
    Ok(())
}

pub(super) fn undo_files(files: &[(PathBuf, PathBuf)]) {
    for (from, to) in files.iter().rev() {
        let _ = fs::rename(to, from);
    }
}

pub(super) fn same_dir(left: &Path, right: &Path) -> bool {
    matches!((left.canonicalize(), right.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

pub(super) fn remove_empty(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                remove_empty(&entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}

fn move_path(src: &Path, dest: &Path, files: &mut Vec<(PathBuf, PathBuf)>) -> Result<(), Value> {
    if !src.exists() {
        return Ok(());
    }
    if dest.exists() {
        return Err(conflict("File already exists"));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| internal(err.to_string()))?;
    }
    fs::rename(src, dest).map_err(|err| internal(err.to_string()))?;
    files.push((src.to_path_buf(), dest.to_path_buf()));
    Ok(())
}
