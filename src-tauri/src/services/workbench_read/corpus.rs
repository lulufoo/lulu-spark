use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{json, Map, Value};

use crate::config::meili_env::workbench_knowledge_root_path;

const CORPUS_LAYERS: &[&str] = &["raw", "distilled", "digest", "trace", "diagnose"];

/// Workbench sidebar index (`corpus/index.json`), replaces static `GET /index.json` via server.py.
pub fn get_corpus_index(_repo_root: &Path) -> Value {
    let corpus = workbench_knowledge_root_path(_repo_root);
    let index_path = corpus.join("index.json");
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

const MAX_CORPUS_FILES_BATCH: usize = 32;

fn load_index_entries(repo_root: &Path) -> Result<Map<String, Value>, Value> {
    let value = get_corpus_index(repo_root);
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

fn entry_has_digest_layer(entry: &Value) -> bool {
    entry
        .get("layers")
        .and_then(|v| v.as_array())
        .is_some_and(|layers| layers.iter().any(|l| l.as_str() == Some("digest")))
}

fn topic_from_common_path(common_path: &str) -> &str {
    common_path.split('/').next().unwrap_or(common_path)
}

fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Slim catalog for MCP: latest digest entry per top-level topic (`id`, `topic`, `created_at` only).
pub fn get_corpus_catalog_latest_per_topic(repo_root: &Path) -> Value {
    let entries = match load_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };

    let mut best_by_topic: HashMap<String, (String, String, String)> = HashMap::new();

    for (id, entry) in &entries {
        if !is_valid_entry_id(id) {
            continue;
        }
        if !entry_has_digest_layer(entry) {
            continue;
        }
        let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
            continue;
        };
        if common_path.is_empty() {
            continue;
        }
        let created_at = entry
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let topic = topic_from_common_path(common_path).to_string();
        match best_by_topic.get(&topic) {
            None => {
                best_by_topic.insert(
                    topic.clone(),
                    (id.clone(), topic, created_at.to_string()),
                );
            }
            Some((_, _, prev_created)) if created_at > prev_created.as_str() => {
                best_by_topic.insert(
                    topic.clone(),
                    (id.clone(), topic, created_at.to_string()),
                );
            }
            _ => {}
        }
    }

    let mut items: Vec<Value> = best_by_topic
        .into_values()
        .map(|(id, topic, created_at)| {
            json!({
                "id": id,
                "topic": topic,
                "created_at": created_at,
            })
        })
        .collect();
    items.sort_by(|a, b| {
        a.get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("topic").and_then(|v| v.as_str()).unwrap_or(""))
    });
    json!({ "items": items })
}

/// Batch-read digest bodies by index entry id.
pub fn get_corpus_files_by_ids(repo_root: &Path, ids: &[String]) -> Value {
    if ids.is_empty() {
        return json!({ "error": "ids must not be empty", "_status": 400 });
    }
    if ids.len() > MAX_CORPUS_FILES_BATCH {
        return json!({
            "error": format!("too many ids (max {MAX_CORPUS_FILES_BATCH})"),
            "_status": 400
        });
    }

    let entries = match load_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };

    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.trim();
        if !is_valid_entry_id(id) {
            items.push(json!({
                "id": id,
                "ok": false,
                "error": "Invalid id",
            }));
            continue;
        }
        let Some(entry) = entries.get(id) else {
            items.push(json!({
                "id": id,
                "ok": false,
                "error": "Entry not found",
            }));
            continue;
        };
        let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
            items.push(json!({
                "id": id,
                "ok": false,
                "error": "Missing common_path",
            }));
            continue;
        };
        let file_value = get_corpus_file(repo_root, "digest", common_path);
        if let Some(status) = file_value.get("_status").and_then(|v| v.as_u64()) {
            let msg = file_value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("read failed");
            items.push(json!({
                "id": id,
                "ok": false,
                "error": format!("HTTP {status}: {msg}"),
            }));
            continue;
        }
        let content = file_value
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        items.push(json!({
            "id": id,
            "ok": true,
            "content": content,
        }));
    }
    json!({ "items": items })
}

/// Corpus markdown body (`{layer}/{common_path}`), replaces static file fetch via server.py.
pub fn get_corpus_file(_repo_root: &Path, layer: &str, common_path: &str) -> Value {
    let layer = layer.trim();
    if !CORPUS_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let common_path = common_path.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let corpus = workbench_knowledge_root_path(_repo_root);
    let target = corpus.join(layer).join(common_path);
    let corpus_canon = match corpus.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    let target_canon = target.canonicalize().unwrap_or(target);
    let prefix = format!(
        "{}{}",
        corpus_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return json!({ "error": "Path traversal not allowed", "_status": 400 });
    }
    if !target_canon.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{common_path}"),
            "_status": 404
        });
    }
    match fs::read_to_string(&target_canon) {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

/// MIME whitelist for corpus raster assets (PNG/JPEG/GIF/WebP).
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

fn corpus_asset_common_path(base: &str, href: &str) -> Result<String, Value> {
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

fn corpus_target_within_root(repo_root: &Path, layer: &str, common_path: &str) -> Result<PathBuf, Value> {
    let corpus = workbench_knowledge_root_path(repo_root);
    let target = corpus.join(layer).join(common_path);
    let corpus_canon = match corpus.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
    let target_canon = target.canonicalize().unwrap_or(target);
    let prefix = format!(
        "{}{}",
        corpus_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return Err(json!({ "error": "Path traversal not allowed", "_status": 400 }));
    }
    Ok(target_canon)
}

/// Corpus binary asset (`{layer}/{dirname(base)}/{relative_href}`), base64 in JSON for invoke.
pub fn get_corpus_asset(repo_root: &Path, layer: &str, base: &str, href: &str) -> Value {
    let layer = layer.trim();
    if !CORPUS_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let common_path = match corpus_asset_common_path(base, href) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let target_canon = match corpus_target_within_root(repo_root, layer, &common_path) {
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
