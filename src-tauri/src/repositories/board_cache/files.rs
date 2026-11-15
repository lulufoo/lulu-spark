//! Directory scans and bounded-cache file maintenance.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{remove_file, BoardCache};

impl BoardCache {
    pub(super) fn entries(&self) -> Result<Vec<CacheFile>, String> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.to_string()),
        };
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !cache_filename(&entry.file_name().to_string_lossy())
                || !entry.file_type().map_err(|e| e.to_string())?.is_file()
            {
                continue;
            }
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            files.push(CacheFile {
                path: entry.path(),
                bytes: metadata.len(),
                modified: metadata.modified().map_err(|e| e.to_string())?,
            });
        }
        Ok(files)
    }

    pub(super) fn remove_interrupted_writes(&self) -> Result<(), String> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.strip_suffix(".tmp").is_some_and(cache_filename)
                && entry.file_type().map_err(|e| e.to_string())?.is_file()
            {
                remove_file(&entry.path())?;
            }
        }
        Ok(())
    }

    pub(super) fn evict_oldest(&self, keep: &Path) -> Result<(), String> {
        let mut files = self.entries()?;
        let mut total: u64 = files.iter().map(|f| f.bytes).sum();
        files.sort_by(|a, b| a.modified.cmp(&b.modified).then_with(|| a.path.cmp(&b.path)));
        for file in files {
            if total <= self.capacity_bytes {
                break;
            }
            if file.path != keep {
                remove_file(&file.path)?;
                total = total.saturating_sub(file.bytes);
            }
        }
        Ok(())
    }
}

pub(super) struct CacheFile {
    pub(super) path: PathBuf,
    pub(super) bytes: u64,
    pub(super) modified: SystemTime,
}

fn cache_filename(name: &str) -> bool {
    name.strip_suffix(".json").is_some_and(|stem| {
        stem.len() == 32
            && stem
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

#[cfg(test)]
#[path = "../../unit-tests/repositories/board_cache/files.rs"]
mod tests;
