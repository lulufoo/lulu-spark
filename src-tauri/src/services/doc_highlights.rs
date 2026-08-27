//! Local document highlight cache under `{cache_dir}/doc-highlights/{md5}.json`.
//! Not written to annotation sidecars or git.

use std::fs;

use md5::{Digest, Md5};
use serde_json::{json, Map, Value};

use crate::config::paths;
use crate::services::id::random_hex12;

const KEY_PREFIXES: [&str; 3] = ["notes:", "knowledge:", "todos:"];

pub fn highlight_key_md5(key: &str) -> String {
    format!("{:x}", Md5::digest(key.as_bytes()))
}

fn validate_key(raw: &str) -> Result<&str, Value> {
    let key = raw.trim();
    if key.is_empty() || key.len() > 512 || key.contains("..") || key.contains('\0') {
        return Err(json!({ "error": "Invalid highlight key", "_status": 400 }));
    }
    if !KEY_PREFIXES.iter().any(|p| key.starts_with(p)) {
        return Err(json!({ "error": "Invalid highlight key", "_status": 400 }));
    }
    Ok(key)
}

fn cache_path(key: &str) -> Result<std::path::PathBuf, Value> {
    let md5 = highlight_key_md5(key);
    paths::doc_highlights_file(&md5)
        .map_err(|_| json!({ "error": "Highlight cache path unavailable", "_status": 500 }))
}

fn read_file(path: &std::path::Path) -> Value {
    if !path.exists() {
        return json!({ "highlights": [] });
    }
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| json!({ "highlights": [] })),
        Err(_) => json!({ "highlights": [] }),
    }
}

fn write_file(path: &std::path::Path, body: &Value) -> Result<(), Value> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
    }
    let tmp = path.with_extension("json.tmp");
    let encoded =
        serde_json::to_string_pretty(body).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
    fs::write(&tmp, encoded).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
    fs::rename(&tmp, path).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
    Ok(())
}

fn highlights_array(root: &mut Value) -> Result<&mut Vec<Value>, Value> {
    if !root.is_object() {
        *root = json!({ "highlights": [] });
    }
    let obj = root
        .as_object_mut()
        .ok_or_else(|| json!({ "error": "Invalid highlight cache", "_status": 500 }))?;
    let entry = obj.entry("highlights".to_string()).or_insert_with(|| json!([]));
    entry
        .as_array_mut()
        .ok_or_else(|| json!({ "error": "Invalid highlight cache", "_status": 500 }))
}

pub fn get_doc_highlights(key: &str) -> Value {
    let key = match validate_key(key) {
        Ok(k) => k,
        Err(e) => return e,
    };
    let path = match cache_path(key) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let mut body = read_file(&path);
    if let Some(obj) = body.as_object_mut() {
        obj.insert("key".into(), json!(key));
        if !obj.contains_key("highlights") {
            obj.insert("highlights".into(), json!([]));
        }
    }
    body
}

pub fn update_doc_highlights(key: &str, highlight: Value, ts: &str) -> Value {
    let key = match validate_key(key) {
        Ok(k) => k,
        Err(e) => return e,
    };
    let path = match cache_path(key) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let hid = highlight
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let text = highlight
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let occurrence = highlight.get("occurrence").cloned();
    let ts = ts.trim().to_string();

    let mut body = read_file(&path);
    let highlights = match highlights_array(&mut body) {
        Ok(arr) => arr,
        Err(e) => return e,
    };

    let out_id = if !hid.is_empty() {
        highlights.retain(|h| h.get("id").and_then(|v| v.as_str()) != Some(hid.as_str()));
        String::new()
    } else {
        if text.is_empty() {
            return json!({
                "error": "text required for new highlight",
                "_status": 400
            });
        }
        let out_id = random_hex12();
        let mut entry = Map::new();
        entry.insert("id".into(), json!(out_id));
        entry.insert("text".into(), json!(text));
        entry.insert("ts".into(), json!(ts));
        if let Some(occ) = occurrence {
            if let Some(n) = occ.as_i64() {
                entry.insert("occurrence".into(), json!(n));
            } else if let Some(n) = occ.as_u64() {
                entry.insert("occurrence".into(), json!(n as i64));
            }
        }
        highlights.push(Value::Object(entry));
        out_id
    };

    if highlights.is_empty() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        return json!({ "ok": true, "id": out_id });
    }

    if let Some(obj) = body.as_object_mut() {
        obj.insert("key".into(), json!(key));
    }
    if let Err(e) = write_file(&path, &body) {
        return e;
    }
    json!({ "ok": true, "id": out_id })
}

#[cfg(test)]
#[path = "../unit-tests/services/doc_highlights.rs"]
mod tests;
