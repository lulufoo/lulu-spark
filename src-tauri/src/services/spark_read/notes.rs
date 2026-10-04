use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{json, Map, Value};

use crate::config::roots::{knowledge_root_string, notes_root_path};

const NOTES_LAYERS: &[&str] = &["raw", "digest"];

fn is_under_root(root: &Path, target: &Path) -> bool {
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let prefix = format!(
        "{}{}",
        root_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    target.to_string_lossy().starts_with(&prefix)
}

/// Notes sidebar index (`notes/index.json`).
pub fn get_notes_index(_repo_root: &Path) -> Value {
    let notes = notes_root_path(_repo_root);
    let index_path = notes.join("index.json");
    if !index_path.is_file() {
        return json!({
            "error": format!("No such file: {}", index_path.display()),
            "_status": 404
        });
    }
    let Ok(text) = fs::read_to_string(&index_path) else {
        return json!({ "error": "failed to read index.json", "_status": 500 });
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => v,
        Err(e) => json!({ "error": format!("invalid index.json: {e}"), "_status": 500 }),
    }
}

pub fn load_notes_index_entries(repo_root: &Path) -> Result<Map<String, Value>, Value> {
    let value = get_notes_index(repo_root);
    if value.get("_status").is_some() {
        return Err(value);
    }
    let entries = value
        .get("entries")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| value.as_object().cloned())
        .ok_or_else(|| json!({ "error": "invalid index entries", "_status": 500 }))?;
    Ok(entries)
}

/// Notes markdown body (`{layer}/{common_path}`).
pub fn get_notes_file(_repo_root: &Path, layer: &str, common_path: &str) -> Value {
    let layer = layer.trim();
    if !NOTES_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let common_path = common_path.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let notes = notes_root_path(_repo_root);
    let target = notes.join(layer).join(common_path);
    let target_canon = target.canonicalize().unwrap_or_else(|_| target.clone());
    if is_under_root(
        Path::new(&knowledge_root_string(_repo_root)),
        &target_canon,
    ) {
        return read_text_file(&target_canon, layer, common_path);
    }
    let notes_canon = match notes.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    if !is_under_root(&notes_canon, &target_canon) {
        return json!({ "error": "Path traversal not allowed", "_status": 400 });
    }
    read_text_file(&target_canon, layer, common_path)
}

fn read_text_file(target: &Path, layer: &str, common_path: &str) -> Value {
    if !target.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{common_path}"),
            "_status": 404
        });
    }
    match fs::read_to_string(target) {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

/// MIME whitelist for notes raster assets (PNG/JPEG/GIF/WebP).
pub fn mime_from_extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn notes_asset_common_path(base: &str, href: &str) -> Result<String, Value> {
    let base = base.trim();
    let href = href.trim();
    if base.is_empty() || href.is_empty() {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if base.contains("..") || href.contains("..") {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("data:")
        || href.starts_with("blob:")
        || href.starts_with('/')
    {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    let joined = Path::new(base)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(href);
    let joined_str = joined.to_string_lossy().replace('\\', "/");
    if joined_str.is_empty() {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    Ok(joined_str)
}

fn notes_target_within_root(repo_root: &Path, layer: &str, common_path: &str) -> Result<PathBuf, Value> {
    let notes = notes_root_path(repo_root);
    let target = notes.join(layer).join(common_path);
    let notes_canon = match notes.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
    let target_canon = target.canonicalize().unwrap_or(target);
    let prefix = format!(
        "{}{}",
        notes_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return Err(json!({ "error": "Path traversal not allowed", "_status": 400 }));
    }
    Ok(target_canon)
}

/// Notes binary asset (`{layer}/{dirname(base)}/{relative_href}`), base64 in JSON for invoke.
pub fn get_notes_asset(repo_root: &Path, layer: &str, base: &str, href: &str) -> Value {
    let layer = layer.trim();
    if !NOTES_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let common_path = match notes_asset_common_path(base, href) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let target_canon = match notes_target_within_root(repo_root, layer, &common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let mime = match mime_from_extension(&target_canon) {
        Some(m) => m,
        None => {
            return json!({
                "error": "Unsupported media type",
                "_status": 400
            });
        }
    };
    if !target_canon.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{common_path}"),
            "_status": 404
        });
    }
    match fs::read(&target_canon) {
        Ok(bytes) => {
            let data_b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            json!({ "data_b64": data_b64, "mime_type": mime })
        }
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}
