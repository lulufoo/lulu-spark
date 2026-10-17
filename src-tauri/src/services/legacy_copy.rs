//! One-time copy of the legacy Notes and Knowledge files into `workspaces/`.
//!
//! The database rows are copied by a migration step; this copies the files they point at:
//!
//! - `notes/{raw,digest}`           → `workspaces/default/{raw,digest}`
//! - `knowledge/{name}` (legacy knowledge id) → `workspaces/knowledge/raw/{project.id}`
//!
//! The copy never overwrites an existing destination file, skips `.git` and symlinks, and
//! writes a marker once it finishes, so an interrupted run is simply resumed on next start.
//!
//! Once the marker exists, [`remove_copied_legacy_files`] deletes every legacy file whose
//! destination holds identical bytes, then the emptied folders. Anything else stays.

use std::fs;
use std::path::Path;

use crate::services::spark_data::{list_projects, DEFAULT_WORKSPACE_ID, KNOWLEDGE_WORKSPACE_ID};

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
    /// Legacy files deleted because the workspace copy holds the same bytes.
    pub removed: usize,
}

/// Copy the legacy files under `data_dir` (`{runtime_data_dir}`).
/// `knowledge` is `(source folder under knowledge/, dest project id)`.
pub fn copy_legacy_files(
    data_dir: &Path,
    knowledge: &[(String, String)],
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
    for (folder, dest_id) in knowledge {
        let src = data_dir.join("knowledge").join(folder);
        if !src.is_dir() {
            report.missing_folders.push(folder.clone());
            continue;
        }
        copy_tree(&src, &knowledge_raw.join(dest_id), &mut report)?;
    }

    fs::create_dir_all(&workspaces).map_err(|e| e.to_string())?;
    fs::write(&marker, b"").map_err(|e| e.to_string())?;
    Ok(report)
}

/// One-time move of the oldest layout: `sediment-kb/` was renamed `knowledge/`.
fn rename_sediment_kb(data_dir: &Path) -> Result<(), String> {
    let dest = data_dir.join("knowledge");
    let src = data_dir.join("sediment-kb");
    if !dest.is_dir() && src.is_dir() {
        fs::rename(&src, &dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn knowledge_source_name(project_id: &str) -> &str {
    project_id.strip_prefix("knowledge:").unwrap_or(project_id)
}

fn knowledge_projects() -> Result<Vec<(String, String)>, String> {
    let conn = crate::services::spark_data::open()?;
    Ok(list_projects(&conn, KNOWLEDGE_WORKSPACE_ID)?
        .into_iter()
        .map(|p| (knowledge_source_name(&p.id).to_string(), p.id))
        .collect())
}

/// Startup entry: read the knowledge project folders from the database, copy, then clean up.
pub fn copy_legacy_files_at_startup(data_dir: &Path) -> Result<LegacyCopyReport, String> {
    let marker = data_dir.join("workspaces").join(LEGACY_COPY_MARKER);
    let mut report = if marker.exists() {
        LegacyCopyReport { already_done: true, ..Default::default() }
    } else {
        rename_sediment_kb(data_dir)?;
        copy_legacy_files(data_dir, &knowledge_projects()?)?
    };
    if marker.exists() {
        report.removed = remove_copied_legacy_files(data_dir, &knowledge_projects()?)?;
    }
    Ok(report)
}

/// Delete the legacy files that the workspace copy already holds, byte for byte, then the
/// folders that became empty. A file without an identical copy, `.git`, symlinks and
/// anything unknown stay where they are.
pub fn remove_copied_legacy_files(
    data_dir: &Path,
    knowledge: &[(String, String)],
) -> Result<usize, String> {
    let workspaces = data_dir.join("workspaces");
    let default_dir = workspaces.join(DEFAULT_WORKSPACE_ID);
    let mut removed = 0;
    for layer in ["raw", "digest"] {
        removed += remove_copied_tree(&data_dir.join("notes").join(layer), &default_dir.join(layer))?;
    }
    let knowledge_raw = workspaces.join(KNOWLEDGE_WORKSPACE_ID).join("raw");
    for (folder, dest_id) in knowledge {
        removed += remove_copied_tree(&data_dir.join("knowledge").join(folder), &knowledge_raw.join(dest_id))?;
    }
    // Folders that held only the removed files; `remove_dir` refuses a folder that is not empty.
    for dir in ["notes", "knowledge"] {
        let _ = fs::remove_dir(data_dir.join(dir));
    }
    Ok(removed)
}

fn remove_copied_tree(src: &Path, dest: &Path) -> Result<usize, String> {
    if !src.is_dir() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_symlink() || entry.file_name() == ".git" {
            continue;
        }
        let to = dest.join(entry.file_name());
        if file_type.is_dir() {
            removed += remove_copied_tree(&entry.path(), &to)?;
        } else if same_bytes(&entry.path(), &to) {
            fs::remove_file(entry.path()).map_err(|e| format!("{}: {e}", entry.path().display()))?;
            removed += 1;
        }
    }
    // Only succeeds when nothing is left inside.
    let _ = fs::remove_dir(src);
    Ok(removed)
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    match (fs::read(a), fs::read(b)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
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
