//! Corpus archive writes for MCP (`archive_document`, `archive_digest`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::repositories::atomic_json;
use crate::services::archive_parse::{is_valid_entry_id, parse_archive_document};
use crate::services::id::random_entry_id;
use crate::services::workbench_read::get_corpus_index;

fn corpus_layer_path(
    corpus: &Path,
    layer: &str,
    common_path: &str,
) -> Result<PathBuf, Value> {
    if common_path.is_empty() || common_path.contains("..") {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if layer != "raw" && layer != "digest" {
        return Err(json!({
            "error": format!("Invalid layer: {layer}"),
            "_status": 400
        }));
    }
    let corpus_canon = match corpus.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
  Ok(corpus_canon.join(layer).join(common_path))
}

fn write_markdown_atomic(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_entries_map(repo_root: &Path) -> Result<(PathBuf, Map<String, Value>), Value> {
    let corpus = workbench_knowledge_root_path(repo_root);
    let index_path = corpus.join("index.json");
    let index_value = get_corpus_index(repo_root);
    if index_value.get("_status").is_some() {
        return Err(index_value);
    }
    let entries = index_value
        .get("entries")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| index_value.as_object().cloned())
        .ok_or_else(|| json!({ "error": "invalid index entries", "_status": 500 }))?;
    Ok((index_path, entries))
}

fn save_entries(index_path: &Path, entries: &Map<String, Value>) -> Result<(), Value> {
    let data = json!({ "entries": entries });
    atomic_json::write_json(index_path, &data).map_err(|e| json!({ "error": e, "_status": 500 }))
}

pub fn archive_document(repo_root: &Path, payload: &Value) -> Value {
    let document = payload
        .get("document")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if document.trim().is_empty() {
        return json!({ "error": "Missing document", "_status": 400 });
    }

    let source_type = payload
        .get("source_type")
        .and_then(|v| v.as_str())
        .unwrap_or("summary")
        .trim()
        .to_string();
    if source_type.is_empty() {
        return json!({ "error": "Invalid source_type", "_status": 400 });
    }

    let parsed = match parse_archive_document(&document) {
        Ok(p) => p,
        Err(e) => {
            return json!({ "error": e.message(), "_status": 400 });
        }
    };

    let corpus = workbench_knowledge_root_path(repo_root);
    let raw_path = match corpus_layer_path(&corpus, "raw", &parsed.common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if raw_path.is_file() {
        return json!({
            "error": format!("File already exists: raw/{}", parsed.common_path),
            "_status": 409
        });
    }

    if let Err(e) = write_markdown_atomic(&raw_path, &document) {
        return json!({ "error": e, "_status": 500 });
    }

    let id = random_entry_id();
    let (index_path, mut entries) = match load_entries_map(repo_root) {
        Ok(v) => v,
        Err(v) => {
            let _ = fs::remove_file(&raw_path);
            return v;
        }
    };

    let mut entry = Map::new();
    entry.insert("common_path".to_string(), json!(parsed.common_path));
    entry.insert("created_at".to_string(), json!(parsed.created_at));
    entry.insert("layers".to_string(), json!(["raw"]));
    entry.insert("source_type".to_string(), json!(source_type));
    entries.insert(id.clone(), Value::Object(entry));

    if let Err(v) = save_entries(&index_path, &entries) {
        let _ = fs::remove_file(&raw_path);
        return v;
    }

    json!({
        "ok": true,
        "id": id,
        "common_path": parsed.common_path,
        "raw_path": format!("raw/{}", parsed.common_path),
        "created_at": parsed.created_at
    })
}

pub fn archive_digest(repo_root: &Path, payload: &Value) -> Value {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if !is_valid_entry_id(id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }

    let digest = payload
        .get("digest")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if digest.trim().is_empty() {
        return json!({ "error": "Missing digest", "_status": 400 });
    }

    let force = payload
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let (index_path, mut entries) = match load_entries_map(repo_root) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let Some(entry_val) = entries.get(id).cloned() else {
        return json!({ "error": "Entry not found", "_status": 404 });
    };
    let Some(common_path) = entry_val.get("common_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing common_path", "_status": 500 });
    };
    if common_path.is_empty() {
        return json!({ "error": "Missing common_path", "_status": 500 });
    }

    let corpus = workbench_knowledge_root_path(repo_root);
    let raw_path = match corpus_layer_path(&corpus, "raw", common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !raw_path.is_file() {
        return json!({
            "error": format!("File not found: raw/{common_path}"),
            "_status": 404
        });
    }

    let digest_path = match corpus_layer_path(&corpus, "digest", common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if digest_path.is_file() && !force {
        return json!({
            "error": format!("File already exists: digest/{common_path}"),
            "_status": 409
        });
    }

    if let Err(e) = write_markdown_atomic(&digest_path, &digest) {
        return json!({ "error": e, "_status": 500 });
    }

    let mut entry_obj = entry_val.as_object().cloned().unwrap_or_default();
    let mut layers: Vec<Value> = entry_obj
        .get("layers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !layers.iter().any(|l| l.as_str() == Some("digest")) {
        layers.push(json!("digest"));
    }
    entry_obj.insert("layers".to_string(), Value::Array(layers));
    entries.insert(id.to_string(), Value::Object(entry_obj));

    if let Err(v) = save_entries(&index_path, &entries) {
        let _ = fs::remove_file(&digest_path);
        return v;
    }

    json!({
        "ok": true,
        "id": id,
        "common_path": common_path,
        "digest_path": format!("digest/{common_path}")
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/archive_write.rs"]
mod tests;
