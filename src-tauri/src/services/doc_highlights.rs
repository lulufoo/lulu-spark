//! Local document highlight cache under `{cache_dir}/doc-highlights/{md5}.json`.
//! Not written to annotation sidecars or git.

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Mutex;

use md5::{Digest, Md5};
use serde_json::{json, Map, Value};

use crate::config::paths;
use crate::services::id::random_hex12;

/// A highlight key is `doc:<document id>`.
const KEY_PREFIXES: [&str; 1] = ["doc:"];
static WRITE_LOCK: Mutex<()> = Mutex::new(());

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

fn read_file(path: &std::path::Path) -> Result<Value, Value> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|_| json!({ "error": "Invalid highlight cache", "_status": 500 })),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(json!({ "highlights": [] })),
        Err(error) => Err(json!({ "error": error.to_string(), "_status": 500 })),
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
    let obj = root
        .as_object_mut()
        .ok_or_else(|| json!({ "error": "Invalid highlight cache", "_status": 500 }))?;
    let entry = obj.entry("highlights".to_string()).or_insert_with(|| json!([]));
    entry
        .as_array_mut()
        .ok_or_else(|| json!({ "error": "Invalid highlight cache", "_status": 500 }))
}

fn lock_cache(path: &Path) -> std::io::Result<File> {
    let cache = path.parent().and_then(Path::parent)
        .ok_or_else(|| std::io::Error::other("Highlight cache directory unavailable"))?;
    fs::create_dir_all(cache)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache.join("doc-highlights.lock"))?;
    lock.lock()?;
    Ok(lock)
}

fn mutate_highlights(
    key: &str,
    update: impl FnOnce(&mut Vec<Value>) -> Result<Value, Value>,
) -> Result<Value, Value> {
    let key = validate_key(key)?;
    let path = cache_path(key)?;
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|_| json!({ "error": "Highlight storage unavailable", "_status": 500 }))?;
    // A separate lock file stays in place when a JSON cache is replaced or deleted.
    let _file_lock =
        lock_cache(&path).map_err(|error| json!({ "error": error.to_string(), "_status": 500 }))?;
    let mut body = read_file(&path)?;
    let highlights = highlights_array(&mut body)?;
    let reply = update(highlights)?;
    if highlights.is_empty() {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(json!({ "error": error.to_string(), "_status": 500 })),
        }
    } else {
        body["key"] = json!(key);
        write_file(&path, &body)?;
    }
    Ok(reply)
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
    let mut body = read_file(&path).unwrap_or_else(|_| json!({ "highlights": [] }));
    if let Some(obj) = body.as_object_mut() {
        obj.insert("key".into(), json!(key));
        let highlights = obj.entry("highlights").or_insert_with(|| json!([]));
        if let Some(highlights) = highlights.as_array_mut() {
            for record in highlights.iter_mut().filter_map(Value::as_object_mut) {
                record.entry("comments").or_insert_with(|| json!([]));
            }
        }
    }
    body
}

pub fn update_doc_highlights(key: &str, highlight: Value, ts: &str) -> Value {
    let hid = highlight
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let text = highlight
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    mutate_highlights(key, |highlights| {
        if !hid.is_empty() {
            highlights.retain(|h| h.get("id").and_then(Value::as_str) != Some(hid));
            return Ok(json!({ "ok": true, "id": "" }));
        }
        if text.is_empty() {
            return Err(json!({
                "error": "text required for new highlight",
                "_status": 400
            }));
        }
        let out_id = random_hex12();
        let mut entry = Map::new();
        entry.insert("id".into(), json!(out_id));
        entry.insert("text".into(), json!(text));
        entry.insert("ts".into(), json!(ts.trim()));
        entry.insert("comments".into(), json!([]));
        if let Some(occ) = highlight.get("occurrence") {
            if let Some(n) = occ.as_i64() {
                entry.insert("occurrence".into(), json!(n));
            } else if let Some(n) = occ.as_u64() {
                entry.insert("occurrence".into(), json!(n as i64));
            }
        }
        highlights.push(Value::Object(entry));
        Ok(json!({ "ok": true, "id": out_id }))
    })
    .unwrap_or_else(|reply| reply)
}

pub fn set_doc_highlight_comment(key: &str, highlight_id: &str, text: &str) -> Value {
    let highlight_id = highlight_id.trim();
    let text = text.trim();
    if highlight_id.is_empty() {
        return json!({ "error": "Highlight ID required", "_status": 400 });
    }
    mutate_highlights(key, |highlights| {
        let record = highlights
            .iter_mut()
            .filter_map(Value::as_object_mut)
            .find(|record| record.get("id").and_then(Value::as_str) == Some(highlight_id))
            .ok_or_else(|| json!({ "error": "Highlight not found", "_status": 404 }))?;
        let comments = record.entry("comments").or_insert_with(|| json!([]));
        let comments = comments
            .as_array_mut()
            .ok_or_else(|| json!({ "error": "Invalid highlight comments", "_status": 500 }))?;
        if text.is_empty() {
            if !comments.is_empty() {
                comments.remove(0);
            }
        } else if let Some(comment) = comments.first_mut() {
            let comment = comment
                .as_object_mut()
                .ok_or_else(|| json!({ "error": "Invalid highlight comment", "_status": 500 }))?;
            comment.insert("text".into(), json!(text));
        } else {
            comments.push(json!({ "id": random_hex12(), "text": text }));
        }
        Ok(json!({ "ok": true, "comments": comments }))
    })
    .unwrap_or_else(|reply| reply)
}

#[cfg(test)]
#[path = "../unit-tests/services/doc_highlights.rs"]
mod tests;
