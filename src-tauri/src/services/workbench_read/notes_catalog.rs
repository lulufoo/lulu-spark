use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use super::notes::{get_notes_file, load_notes_index_entries};

const MAX_NOTES_FILES_BATCH: usize = 32;

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
pub fn get_notes_catalog_latest_per_topic(repo_root: &Path) -> Value {
    let entries = match load_notes_index_entries(repo_root) {
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
pub fn get_notes_files_by_ids(repo_root: &Path, ids: &[String]) -> Value {
    if ids.is_empty() {
        return json!({ "error": "ids must not be empty", "_status": 400 });
    }
    if ids.len() > MAX_NOTES_FILES_BATCH {
        return json!({
            "error": format!("too many ids (max {MAX_NOTES_FILES_BATCH})"),
            "_status": 400
        });
    }

    let entries = match load_notes_index_entries(repo_root) {
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
        let file_value = get_notes_file(repo_root, "digest", common_path);
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
