use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::services::annotation::read_annotation_object;
use crate::services::tags_registry::read_registry;

pub fn get_annotation(repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    let corpus = workbench_knowledge_root_path(repo_root);
    read_annotation_object(&corpus, common_path)
}

fn resolve_tags(registry: &Value, tag_keys: &[Value]) -> Value {
    let reg_keys = registry.get("keys").and_then(|v| v.as_object());
    let tags: Vec<Value> = tag_keys
        .iter()
        .filter_map(|v| v.as_str())
        .map(|key| {
            if let Some(entry) = reg_keys.and_then(|m| m.get(key)) {
                let value = entry
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                json!({ "key": key, "value": value })
            } else {
                json!({ "key": key, "value": "未知标签", "unknown": true })
            }
        })
        .collect();
    Value::Array(tags)
}

pub fn get_annotations_summary(repo_root: &Path) -> Value {
    let corpus = workbench_knowledge_root_path(repo_root);
    let registry = read_registry(&corpus);
    let index_path = corpus.join("index.json");
    let Ok(text) = fs::read_to_string(&index_path) else {
        return json!({ "error": format!("No such file: {}", index_path.display()) });
    };
    let Ok(index_data) = serde_json::from_str::<Value>(&text) else {
        return json!({ "error": "invalid index.json" });
    };
    let entries = index_data
        .get("entries")
        .cloned()
        .unwrap_or(index_data.clone());
    let entry_values: Vec<Value> = match &entries {
        Value::Object(m) => m.values().cloned().collect(),
        Value::Array(a) => a.clone(),
        _ => return json!({ "error": "invalid index entries" }),
    };
    let mut result = Map::new();
    for entry in entry_values {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let common_path = obj
            .get("common_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if common_path.is_empty() {
            continue;
        }
        let ann = read_annotation_object(&corpus, &common_path);
        let Some(ann_obj) = ann.as_object() else {
            continue;
        };
        if ann_obj.is_empty() {
            continue;
        }
        let mut summary = Map::new();
        if ann_obj.get("done").and_then(|v| v.as_bool()) == Some(true) {
            summary.insert("done".into(), json!(true));
        }
        if let Some(imp) = ann_obj.get("importance").and_then(|v| v.as_str()) {
            if matches!(imp, "high" | "medium" | "low") {
                summary.insert("importance".into(), json!(imp));
            }
        }
        if let Some(links) = ann_obj.get("links") {
            if !links.is_null() {
                summary.insert("links".into(), links.clone());
            }
        }
        if let Some(tag_keys) = ann_obj.get("tag_keys").and_then(|v| v.as_array()) {
            if !tag_keys.is_empty() {
                summary.insert("tag_keys".into(), Value::Array(tag_keys.clone()));
                summary.insert("tags".into(), resolve_tags(&registry, tag_keys));
            }
        }
        for layer in ["raw", "distilled", "digest", "trace", "diagnose"] {
            let Some(layer_data) = ann_obj.get(layer).and_then(|v| v.as_object()) else {
                continue;
            };
            let Some(comments) = layer_data.get("comments").and_then(|c| c.as_array()) else {
                continue;
            };
            if !comments.is_empty() {
                let cc = summary
                    .entry("comment_counts")
                    .or_insert_with(|| json!({}));
                if let Some(m) = cc.as_object_mut() {
                    m.insert(layer.to_string(), json!(comments.len()));
                }
            }
        }
        if !summary.is_empty() {
            result.insert(common_path, Value::Object(summary));
        }
    }
    Value::Object(result)
}
