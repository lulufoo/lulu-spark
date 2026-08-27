use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::services::archive_parse::is_valid_entry_id;

use super::store::{corpus_layer_path, load_entries_map, save_entries, write_markdown_atomic};

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
