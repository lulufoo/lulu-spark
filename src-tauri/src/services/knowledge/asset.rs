use std::fs;
use std::path::Path;

use base64::Engine;
use serde_json::{json, Value};

use crate::repositories::knowledge::kb_safe_path;
use crate::services::spark_read::{knowledge_root_string, mime_from_extension};

fn err_status_code(msg: &str) -> u16 {
    if msg.contains("invalid") || msg.contains("traversal") || msg.contains("required") {
        400
    } else {
        404
    }
}

fn kb_asset_rel_path(base: &str, href: &str) -> Result<String, Value> {
    let base = base.trim();
    let href = href.trim();
    if base.is_empty() || href.is_empty() {
        return Err(json!({ "error": "invalid path", "_status": 400 }));
    }
    if base.contains("..") || href.contains("..") {
        return Err(json!({ "error": "invalid path", "_status": 400 }));
    }
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("data:")
        || href.starts_with("blob:")
        || href.starts_with('/')
    {
        return Err(json!({ "error": "invalid path", "_status": 400 }));
    }
    let joined = Path::new(base)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(href);
    let joined_str = joined.to_string_lossy().replace('\\', "/");
    if joined_str.is_empty() {
        return Err(json!({ "error": "invalid path", "_status": 400 }));
    }
    Ok(joined_str)
}

/// Knowledge binary asset (`{dirname(base)}/{relative_href}`), base64 in JSON for invoke.
pub fn kb_asset_json(repo_root: &Path, repo: &str, base: &str, href: &str) -> Value {
    let rel = match kb_asset_rel_path(base, href) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let kb_root_str = knowledge_root_string(repo_root);
    let kb_root = Path::new(&kb_root_str);
    match kb_safe_path(kb_root, repo, &rel) {
        Err(e) => json!({ "error": e, "_status": err_status_code(&e) }),
        Ok(target) => {
            let mime = match mime_from_extension(&target) {
                Some(m) => m,
                None => {
                    return json!({
                        "error": "Unsupported media type",
                        "_status": 400
                    });
                }
            };
            if !target.is_file() {
                let e = format!("file not found: {rel}");
                return json!({ "error": e, "_status": 404 });
            }
            match fs::read(&target) {
                Ok(bytes) => {
                    let data_b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                    json!({ "data_b64": data_b64, "mime_type": mime })
                }
                Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
            }
        }
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/asset.rs"]
mod tests;
