//! App-internal note write paths → keep the index in step without a full rebuild.
//!
//! Scope (2026-09-05 /converge): create / save / delete / move done by the App
//! itself. Files edited outside the App still rely on the startup / manual rebuild.

use std::path::Path;

use super::collect::note_file_chunks;
use super::query::cache_dir_or_err;
use super::store::{index_exists, open, replace_path_chunks};

/// `(layer, common_path)` pairs. Existing files are re-chunked and replace their
/// rows; missing files have their rows dropped. No-op when no index file exists.
pub fn sync_note_files(repo_root: &Path, items: &[(&str, &str)]) -> Result<usize, String> {
    let cache = cache_dir_or_err()?;
    if !index_exists(&cache) {
        return Ok(0);
    }
    let conn = open(&cache)?;
    let mut touched = 0;
    for (layer, common_path) in items {
        let (key, chunks) = note_file_chunks(repo_root, layer, common_path);
        replace_path_chunks(&conn, &key, &chunks)?;
        touched += 1;
    }
    Ok(touched)
}

/// Same as [`sync_note_files`] but never fails the caller's write response.
pub fn sync_note_files_best_effort(repo_root: &Path, items: &[(&str, &str)]) {
    if let Err(e) = sync_note_files(repo_root, items) {
        eprintln!("keyword index sync skipped: {e}");
    }
}
