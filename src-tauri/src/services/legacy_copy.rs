//! One-time copy of the legacy Notes and Knowledge files into `workspaces/`.
//!
//! The database rows are copied by the last migration step; this copies the files they
//! point at, so the old folders stay untouched until the cleanup issue removes them:
//!
//! - `notes/{raw,digest}`           → `workspaces/default/{raw,digest}`
//! - `knowledge/{folder}` (per `knowledge_directory` row) → `workspaces/knowledge/raw/{folder}`
//!
//! The copy never overwrites an existing destination file, skips `.git` and symlinks, and
//! writes a marker once it finishes, so an interrupted run is simply resumed on next start.

use std::fs;
use std::path::Path;

use crate::services::spark_data::{
    list_knowledge_directories, DEFAULT_WORKSPACE_ID, KNOWLEDGE_WORKSPACE_ID,
};

/// Marker written under `workspaces/` once the copy completed.
pub const LEGACY_COPY_MARKER: &str = ".legacy_copied";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LegacyCopyReport {
    /// The marker already existed, so nothing was done.
    pub already_done: bool,
    pub copied: usize,
    /// Destination already had the file; left as it was.
    pub skipped_existing: usize,
    /// Knowledge folders that are registered but absent on disk.
    pub missing_folders: Vec<String>,
}

/// Copy the legacy files under `data_dir` (`{runtime_data_dir}`) for the given knowledge folders.
pub fn copy_legacy_files(
    data_dir: &Path,
    knowledge_folders: &[String],
) -> Result<LegacyCopyReport, String> {
    let workspaces = data_dir.join("workspaces");
    let marker = workspaces.join(LEGACY_COPY_MARKER);
    let mut report = LegacyCopyReport::default();
    if marker.exists() {
        report.already_done = true;
        return Ok(report);
    }

    let default_dir = workspaces.join(DEFAULT_WORKSPACE_ID);
    for layer in ["raw", "digest"] {
        copy_tree(&data_dir.join("notes").join(layer), &default_dir.join(layer), &mut report)?;
    }
    let knowledge_raw = workspaces.join(KNOWLEDGE_WORKSPACE_ID).join("raw");
    for folder in knowledge_folders {
        let src = data_dir.join("knowledge").join(folder);
        if !src.is_dir() {
            report.missing_folders.push(folder.clone());
            continue;
        }
        copy_tree(&src, &knowledge_raw.join(folder), &mut report)?;
    }

    fs::create_dir_all(&workspaces).map_err(|e| e.to_string())?;
    fs::write(&marker, b"").map_err(|e| e.to_string())?;
    Ok(report)
}

/// Startup entry: read the registered knowledge folders from the database, then copy.
pub fn copy_legacy_files_at_startup(data_dir: &Path) -> Result<LegacyCopyReport, String> {
    if data_dir.join("workspaces").join(LEGACY_COPY_MARKER).exists() {
        return Ok(LegacyCopyReport { already_done: true, ..Default::default() });
    }
    let conn = crate::services::spark_data::open()?;
    let folders: Vec<String> =
        list_knowledge_directories(&conn)?.into_iter().map(|d| d.folder).collect();
    copy_legacy_files(data_dir, &folders)
}

fn copy_tree(src: &Path, dest: &Path, report: &mut LegacyCopyReport) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if file_type.is_symlink() || name == ".git" {
            continue;
        }
        let to = dest.join(&name);
        if file_type.is_dir() {
            copy_tree(&entry.path(), &to, report)?;
        } else if to.exists() {
            report.skipped_existing += 1;
        } else {
            fs::copy(entry.path(), &to)
                .map_err(|e| format!("{} → {}: {e}", entry.path().display(), to.display()))?;
            report.copied += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../unit-tests/services/legacy_copy.rs"]
mod tests;
