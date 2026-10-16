use std::path::Path;

use serde_json::{json, Map, Value};

use crate::services::annotation::read_annotation_object;
use crate::services::spark_data;
use crate::services::tags_registry::read_registry;

pub fn get_annotation(_repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    read_annotation_object(common_path)
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

pub fn get_annotations_summary(_repo_root: &Path) -> Value {
    let registry = read_registry();
    let notes = match spark_data::open().and_then(|conn| spark_data::list_notes(&conn)) {
        Ok(notes) => notes,
        Err(e) => return json!({ "error": format!("annotations: {e}"), "_status": 500 }),
    };
    let mut result = Map::new();
    for note in notes {
        let common_path = note.raw_path.clone();
        if common_path.is_empty() {
            continue;
        }
        let ann = spark_data::annotation_object(&note);
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
        for layer in ["raw", "digest"] {
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
