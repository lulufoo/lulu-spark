//! Generic absolute-path file read/write for FilePopup. Not a Notes or Knowledge API.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

fn invalid(msg: &str, status: u16) -> Value {
    json!({ "error": msg, "_status": status })
}

fn resolve_abs_file(path: &str) -> Result<PathBuf, Value> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err(invalid("Invalid path", 400));
    }
    if raw.contains('\0') {
        return Err(invalid("Invalid path", 400));
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(invalid("Path must be absolute", 400));
    }
    let canon = path.canonicalize().map_err(|e| invalid(&e.to_string(), 404))?;
    if !canon.is_file() {
        return Err(invalid("File not found", 404));
    }
    Ok(canon)
}

pub fn read_abs_file(path: &str) -> Value {
    let target = match resolve_abs_file(path) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match fs::read_to_string(&target) {
        Ok(content) => json!({ "content": content }),
        Err(e) => invalid(&e.to_string(), 500),
    }
}

pub fn write_abs_file(path: &str, content: &str) -> Value {
    let target = match resolve_abs_file(path) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match fs::write(&target, content) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => invalid(&e.to_string(), 500),
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/abs_file.rs"]
mod tests;
