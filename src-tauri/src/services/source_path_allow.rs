//! Shared allow-list resolution for Host reads of local source files (todo attachments, archive).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::config::paths;

/// Max bytes for a source `.md` copied into todo attachments (2 MiB).
pub const MAX_ATTACHMENT_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

/// Max bytes for a source `.md` archived via `create_note` (8 MiB).
pub const MAX_ARCHIVE_SOURCE_BYTES: u64 = 8 * 1024 * 1024;

fn push_root(roots: &mut Vec<PathBuf>, p: PathBuf) {
    if !p.exists() {
        let _ = fs::create_dir_all(&p);
    }
    if let Ok(canonical) = p.canonicalize() {
        if !roots.iter().any(|r| r == &canonical) {
            roots.push(canonical);
        }
    }
}

/// Canonical roots under which `source_path` may reside.
pub fn allow_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(p) = paths::knowledge_root() {
        push_root(&mut roots, p);
    }
    if let Ok(p) = paths::spark_root() {
        push_root(&mut roots, p);
    }
    push_root(&mut roots, paths::runtime_data_dir());
    if let Ok(p) = paths::cache_dir() {
        push_root(&mut roots, p);
    }
    if let Ok(repo) = paths::repo_root() {
        push_root(&mut roots, repo.join(".cache"));
    }
    push_root(&mut roots, std::env::temp_dir());
    roots
}

fn path_is_under_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// Resolve an absolute source path under the allow-list.
/// Does not enforce `.md` or basename rules — callers apply domain constraints.
pub fn resolve_allowed_source_path(
    source_path: &str,
    max_bytes: u64,
) -> Result<PathBuf, Value> {
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
    let roots = allow_roots();
    if !roots
        .iter()
        .any(|root| path_is_under_root(&canonical, root))
    {
        return Err(json!({ "error": "source_path not allowed", "_status": 403 }));
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
