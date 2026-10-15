//! Delete the old files once reconciliation passed.

use std::fs;
use std::path::{Path, PathBuf};

pub fn delete_files(files: &[PathBuf]) -> Result<(), String> {
    for file in files {
        fs::remove_file(file).map_err(|e| format!("delete {}: {e}", file.display()))?;
    }
    Ok(())
}

/// Remove empty folders below `dir`, then `dir` itself when nothing is left in it.
pub fn prune_empty_dirs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            prune_empty_dirs(&entry.path());
        }
    }
    let _ = fs::remove_dir(dir);
}
