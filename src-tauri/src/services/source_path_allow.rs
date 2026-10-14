//! Source path resolution for Host reads of local files: no allow-list; callers apply domain constraints.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

/// Max bytes for a source `.md` copied into todo attachments (2 MiB).
pub const MAX_ATTACHMENT_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

/// Max bytes for a source `.md` archived via `create_note` (8 MiB).
pub const MAX_ARCHIVE_SOURCE_BYTES: u64 = 8 * 1024 * 1024;

/// Resolve an absolute source path to a canonical regular file within `max_bytes`.
/// Does not enforce `.md` / basename rules — callers apply domain constraints.
pub fn resolve_source_path(source_path: &str, max_bytes: u64) -> Result<PathBuf, Value> {
    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Err(json!({ "error": "Missing source_path", "_status": 400 }));
    }
    let raw = PathBuf::from(trimmed);
    if !raw.is_absolute() {
        return Err(json!({ "error": "Invalid source_path", "_status": 400 }));
    }
    if !raw.exists() {
        return Err(json!({ "error": "Source file not found", "_status": 404 }));
    }
    let canonical = match raw.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Err(json!({ "error": "Invalid source_path", "_status": 400 }));
        }
    };
    if !canonical.is_file() {
        return Err(json!({ "error": "Source is not a regular file", "_status": 400 }));
    }
    let meta = match fs::metadata(&canonical) {
        Ok(m) => m,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
    if meta.len() > max_bytes {
        return Err(json!({ "error": "Source file too large", "_status": 413 }));
    }
    Ok(canonical)
}

#[cfg(test)]
#[path = "../unit-tests/services/source_path_allow.rs"]
mod tests;
