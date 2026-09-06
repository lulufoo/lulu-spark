//! Companion raster assets for desktop `create_note` (`asset_paths`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::source_path_allow;

pub(super) const MAX_NOTE_ASSET_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_NOTE_ASSETS: usize = 8;

pub(super) struct ResolvedAsset {
    pub source: PathBuf,
    pub rel: String,
    pub dest: PathBuf,
}

pub(super) fn reject_unsupported_asset_paths(payload: &Value) -> Option<Value> {
    if payload.get("asset_paths").is_some() {
        Some(json!({
            "error": "asset_paths is not supported",
            "_status": 400
        }))
    } else {
        None
    }
}

pub(super) fn resolve_create_assets(
    payload: &Value,
    raw_md_path: &Path,
    common_path: &str,
) -> Result<Vec<ResolvedAsset>, Value> {
    let Some(arr) = payload.get("asset_paths") else {
        return Ok(Vec::new());
    };
    let Some(items) = arr.as_array() else {
        return Err(json!({
            "error": "asset_paths must be an array",
            "_status": 400
        }));
    };
    if items.len() > MAX_NOTE_ASSETS {
        return Err(json!({
            "error": format!("asset_paths exceeds max {MAX_NOTE_ASSETS}"),
            "_status": 400
        }));
    }
    let parent = raw_md_path.parent().ok_or_else(|| {
        json!({ "error": "Invalid raw path", "_status": 500 })
    })?;
    let dir_rel = Path::new(common_path)
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| json!({ "error": "Invalid common_path", "_status": 400 }))?;

    let mut out = Vec::with_capacity(items.len());
    let mut seen = HashSet::new();
    for item in items {
        let Some(raw) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(json!({
                "error": "Invalid asset_paths item",
                "_status": 400
            }));
        };
        let source = resolve_asset_source(raw)?;
        let dest_name = dest_basename(&source)?;
        if !seen.insert(dest_name.clone()) {
            return Err(json!({
                "error": format!("File already exists: raw/{dir_rel}/{dest_name}"),
                "_status": 409
            }));
        }
        let dest = parent.join(&dest_name);
        if dest.exists() {
            return Err(json!({
                "error": format!("File already exists: raw/{dir_rel}/{dest_name}"),
                "_status": 409
            }));
        }
        out.push(ResolvedAsset {
            source,
            rel: format!("raw/{dir_rel}/{dest_name}"),
            dest,
        });
    }
    Ok(out)
}

pub(super) fn copy_assets(
    assets: &[ResolvedAsset],
    written: &mut Vec<PathBuf>,
) -> Result<(), Value> {
    for asset in assets {
        if let Err(e) = fs::copy(&asset.source, &asset.dest) {
            return Err(json!({ "error": e.to_string(), "_status": 500 }));
        }
        written.push(asset.dest.clone());
    }
    Ok(())
}

fn resolve_asset_source(path: &str) -> Result<PathBuf, Value> {
    match source_path_allow::resolve_allowed_source_path(path, MAX_NOTE_ASSET_BYTES) {
        Ok(canonical) => {
            let ext = canonical
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
                return Err(json!({
                    "error": "Unsupported media type",
                    "_status": 400
                }));
            }
            Ok(canonical)
        }
        Err(err) => Err(remap_source_error(err)),
    }
}

fn dest_basename(source: &Path) -> Result<String, Value> {
    let name = source
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| json!({ "error": "Invalid asset filename", "_status": 400 }))?;
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(json!({ "error": "Invalid asset filename", "_status": 400 }));
    }
    Ok(name.to_string())
}

fn remap_source_error(err: Value) -> Value {
    let Some(msg) = err.get("error").and_then(|v| v.as_str()) else {
        return err;
    };
    let mapped = if msg == "Missing source_path" || msg == "Invalid source_path" {
        "Invalid asset_paths item"
    } else if msg == "source_path not allowed" {
        "asset_paths not allowed"
    } else if msg == "Source file too large" {
        "Asset file too large"
    } else if msg == "Source file not found" {
        "Asset file not found"
    } else {
        return err;
    };
    let mut out = err;
    out["error"] = json!(mapped);
    out
}
