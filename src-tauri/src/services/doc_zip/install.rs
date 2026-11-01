//! Swap an unzipped package in for the local files of a document, or put everything back.

use std::fs;
use std::path::{Path, PathBuf};

use crate::services::docs::attached::{attached_dir, ATTACHED_DIR_NAME};
use crate::services::docs::store::doc_file_path;
use crate::services::spark_data::{translation_common_paths, DocRow};

use super::error_text;
use super::manifest::{Manifest, ATTACHED_DIR, DIGEST_DIR, RAW_DIR};

/// `staging/new` holds the unzipped package. Old files wait in `staging/old` until the new ones
/// are in place. On success the caller removes `staging`; on failure this function has moved every
/// file back and removed the folders it made.
pub(super) fn install(
    root: &Path,
    project_id: &str,
    manifest: &Manifest,
    staging: &Path,
    existing: Option<&DocRow>,
) -> Result<(), String> {
    let new_dir = staging.join("new");
    let mut incoming: Vec<(PathBuf, PathBuf)> = Vec::new();
    for rel in manifest.raw_rels() {
        incoming.push((new_dir.join(RAW_DIR).join(&rel), target(root, "raw", project_id, &rel)?));
    }
    if let Some(rel) = &manifest.digest_path {
        incoming.push((new_dir.join(DIGEST_DIR).join(rel), target(root, "digest", project_id, rel)?));
    }
    let raw_target = target(root, "raw", project_id, &manifest.raw_path)?;
    let new_attached = attached_dir(parent_of(&raw_target)?, &manifest.doc_id).map_err(error_text)?;
    if new_dir.join(ATTACHED_DIR).is_dir() {
        incoming.push((new_dir.join(ATTACHED_DIR), new_attached.clone()));
    }
    let outgoing = outgoing(root, &manifest.doc_id, existing, new_attached)?;

    let mut journal = Journal::default();
    let swapped = swap(&mut journal, &outgoing, &incoming, &staging.join("old"));
    if let Err(error) = swapped {
        let failures = journal.roll_back();
        return Err(if failures.is_empty() {
            error
        } else {
            format!("{error}; and the old files could not all be put back: {}", failures.join("; "))
        });
    }
    for (path, _) in &journal.backups {
        let attached_parent =
            path.parent().filter(|p| p.file_name().is_some_and(|n| n == ATTACHED_DIR_NAME));
        if let Some(parent) = attached_parent {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
}

/// Every local path the document owns now: body, translations, digest, attachment folder. The
/// attachment folder of the new location is included: the doc id owns it wherever it is.
fn outgoing(
    root: &Path,
    doc_id: &str,
    existing: Option<&DocRow>,
    new_attached: PathBuf,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    if let Some(row) = existing {
        let pid = &row.project_id;
        let raw = target(root, "raw", pid, &row.raw_path)?;
        paths.push(attached_dir(parent_of(&raw)?, doc_id).map_err(error_text)?);
        paths.push(raw);
        for (_lang, rel) in translation_common_paths(row)? {
            paths.push(target(root, "raw", pid, &rel)?);
        }
        let digest = row.digest_path.as_deref().unwrap_or(&row.raw_path);
        paths.push(target(root, "digest", pid, digest)?);
    }
    paths.push(new_attached);
    let mut unique: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    Ok(unique)
}

fn swap(
    journal: &mut Journal,
    outgoing: &[PathBuf],
    incoming: &[(PathBuf, PathBuf)],
    old_dir: &Path,
) -> Result<(), String> {
    fs::create_dir_all(old_dir).map_err(|e| format!("{}: {e}", old_dir.display()))?;
    for (n, path) in outgoing.iter().filter(|p| p.exists()).enumerate() {
        journal.back_up(path, &old_dir.join(n.to_string()))?;
    }
    for (staged, destination) in incoming {
        journal.place(staged, destination)?;
    }
    Ok(())
}

fn target(root: &Path, layer: &str, project_id: &str, rel: &str) -> Result<PathBuf, String> {
    doc_file_path(root, layer, project_id, rel).map_err(error_text)
}

fn parent_of(path: &Path) -> Result<&Path, String> {
    path.parent().ok_or_else(|| format!("{} has no folder", path.display()))
}

/// What a swap has done so far, so that it can be undone.
#[derive(Default)]
struct Journal {
    /// `(where the old file was, where it waits)`.
    backups: Vec<(PathBuf, PathBuf)>,
    placed: Vec<PathBuf>,
    /// Folders this swap created, shallowest first.
    made_dirs: Vec<PathBuf>,
}

impl Journal {
    fn back_up(&mut self, path: &Path, slot: &Path) -> Result<(), String> {
        fs::rename(path, slot).map_err(|e| format!("{}: {e}", path.display()))?;
        self.backups.push((path.to_path_buf(), slot.to_path_buf()));
        Ok(())
    }

    /// Move `staged` to `destination`, which must be free: a file already there belongs to
    /// something else, because the files of this document were moved away first.
    fn place(&mut self, staged: &Path, destination: &Path) -> Result<(), String> {
        if destination.exists() {
            return Err(format!("{} is already used by another file", destination.display()));
        }
        if let Some(parent) = destination.parent() {
            self.make_dir(parent)?;
        }
        fs::rename(staged, destination).map_err(|e| format!("{}: {e}", destination.display()))?;
        self.placed.push(destination.to_path_buf());
        Ok(())
    }

    fn make_dir(&mut self, dir: &Path) -> Result<(), String> {
        let mut missing = Vec::new();
        let mut cursor = dir;
        while !cursor.exists() {
            missing.push(cursor.to_path_buf());
            match cursor.parent() {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
        missing.reverse();
        self.made_dirs.extend(missing);
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))
    }

    /// Remove what was placed, remove the folders made, move the old files back. Returns the
    /// steps that failed.
    fn roll_back(self) -> Vec<String> {
        let mut failures = Vec::new();
        for path in self.placed.iter().rev() {
            let removed = if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
            if let Err(e) = removed {
                failures.push(format!("{}: {e}", path.display()));
            }
        }
        for dir in self.made_dirs.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
        for (path, slot) in self.backups.iter().rev() {
            let restored = path
                .parent()
                .map_or(Ok(()), fs::create_dir_all)
                .and_then(|()| fs::rename(slot, path));
            if let Err(e) = restored {
                failures.push(format!("{}: {e}", path.display()));
            }
        }
        failures
    }
}
