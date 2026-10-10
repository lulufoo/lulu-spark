//! Versioned copy of a Host-owned file into a caller-chosen directory.
//!
//! The caller names the directory. The Host enforces only a floor: absolute path,
//! no `..`, resolves (symlinks included) inside the `source_path` allow-list, and
//! never inside the Data directory. Existing files are never overwritten.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::config::paths;
use crate::services::source_path_allow::allow_roots;

fn reject(error: &str, status: u16) -> Value {
    json!({ "error": error, "_status": status })
}

/// Canonicalize the deepest existing ancestor of `raw`, then re-attach the missing tail.
fn resolve_dest_dir(raw: &Path) -> Result<PathBuf, Value> {
    let mut tail: Vec<OsString> = Vec::new();
    let mut current = raw;
    loop {
        if current.exists() {
            let base = current
                .canonicalize()
                .map_err(|_| reject("Invalid dest_dir", 400))?;
            return Ok(tail.iter().rev().fold(base, |acc, part| acc.join(part)));
        }
        match (current.file_name(), current.parent()) {
            (Some(name), Some(parent)) => {
                tail.push(name.to_os_string());
                current = parent;
            }
            _ => return Err(reject("Invalid dest_dir", 400)),
        }
    }
}

fn validate_dest_dir(dest_dir: &str) -> Result<PathBuf, Value> {
    let trimmed = dest_dir.trim();
    if trimmed.is_empty() {
        return Err(reject("Missing dest_dir", 400));
    }
    let raw = PathBuf::from(trimmed);
    if !raw.is_absolute() {
        return Err(reject("dest_dir must be an absolute path", 400));
    }
    if raw.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(reject("dest_dir must not contain '..'", 400));
    }
    let resolved = resolve_dest_dir(&raw)?;
    // `allow_roots` also creates the Data dir, so it can be canonicalized below.
    let roots = allow_roots();
    let data = paths::runtime_data_dir();
    let data = data.canonicalize().unwrap_or(data);
    if resolved.starts_with(&data) {
        return Err(reject("dest_dir must not be inside the Data directory", 403));
    }
    if !roots.iter().any(|root| resolved.starts_with(root)) {
        return Err(reject("dest_dir not allowed", 403));
    }
    Ok(resolved)
}

fn is_safe_stem(stem: &str) -> bool {
    !stem.is_empty() && !stem.contains("..") && !stem.contains('/') && !stem.contains('\\')
}

fn target_name(stem: &str, ext: &str, version: u32) -> String {
    if version == 0 {
        format!("{stem}.{ext}")
    } else {
        format!("{stem}-v{version}.{ext}")
    }
}

/// Copy `src` into `dest_dir` as `<stem>.<ext>`, or `<stem>-v1.<ext>`, `-v2`… when taken.
/// Returns the full path written.
pub fn copy_into_dir_versioned(
    src: &Path,
    dest_dir: &str,
    stem: &str,
    ext: &str,
) -> Result<PathBuf, Value> {
    if !is_safe_stem(stem) {
        return Err(reject("Invalid file name", 400));
    }
    let dir = validate_dest_dir(dest_dir)?;
    fs::create_dir_all(&dir).map_err(|e| reject(&format!("Cannot create dest_dir: {e}"), 500))?;
    let mut version = 0u32;
    loop {
        let target = dir.join(target_name(stem, ext, version));
        match OpenOptions::new().write(true).create_new(true).open(&target) {
            Ok(mut out) => {
                let copied = fs::File::open(src).and_then(|mut input| io::copy(&mut input, &mut out));
                if let Err(e) = copied {
                    let _ = fs::remove_file(&target);
                    return Err(reject(&format!("Cannot copy file: {e}"), 500));
                }
                return Ok(target);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => version += 1,
            Err(e) => return Err(reject(&format!("Cannot write file: {e}"), 500)),
        }
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/file_export.rs"]
mod tests;
