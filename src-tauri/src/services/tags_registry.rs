//! Tags registry (`notes/tags/registry.json`) — refs, reconcile.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::config::roots::notes_root_path;
use crate::repositories::annotation_paths::annotation_json_path;
use crate::repositories::atomic_json;
use crate::services::annotation::read_annotation_object;

pub fn registry_path(notes: &Path) -> PathBuf {
    notes.join("tags/registry.json")
}

pub fn empty_registry() -> Value {
    json!({ "keys": {} })
}

pub fn read_registry(notes: &Path) -> Value {
    let path = registry_path(notes);
    if !path.is_file() {
        return empty_registry();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return empty_registry();
    };
    serde_json::from_str(&text).unwrap_or_else(|_| empty_registry())
}

pub fn save_registry(notes: &Path, registry: &Value) -> Option<Value> {
    let path = registry_path(notes);
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Some(json!({ "error": e.to_string(), "_status": 500 }));
        }
    }
    atomic_json::write_json(&path, registry)
        .err()
        .map(|e| json!({ "error": e, "_status": 500 }))
}

fn keys_map_mut(registry: &mut Value) -> Option<&mut Map<String, Value>> {
    registry.get_mut("keys")?.as_object_mut()
}

pub fn adjust_refs(registry: &mut Value, keys: &[String], delta: i32) {
    if delta == 0 || keys.is_empty() {
        return;
    }
    let Some(map) = keys_map_mut(registry) else {
        return;
    };
    for key in keys {
        if key.is_empty() {
            continue;
        }
        if delta > 0 {
            let entry = map
                .entry(key.clone())
                .or_insert_with(|| json!({ "value": "", "refs": 0 }));
            if let Some(obj) = entry.as_object_mut() {
                let refs = obj.get("refs").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                obj.insert("refs".into(), json!(refs + delta));
            }
        } else {
            let Some(entry) = map.get_mut(key) else {
                continue;
            };
            let Some(obj) = entry.as_object_mut() else {
                continue;
            };
            let refs = obj.get("refs").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let new_refs = refs + delta;
            if new_refs <= 0 {
                map.remove(key);
            } else {
                obj.insert("refs".into(), json!(new_refs));
            }
        }
    }
}

pub fn remove_key_if_zero(registry: &mut Value, key: &str) {
    let Some(map) = keys_map_mut(registry) else {
        return;
    };
    let remove = map
        .get(key)
        .and_then(|v| v.get("refs"))
        .and_then(|v| v.as_i64())
        .map(|r| r <= 0)
        .unwrap_or(false);
    if remove {
        map.remove(key);
    }
}

fn index_common_paths(notes: &Path, index_path: &Path) -> Option<Vec<String>> {
    let text = fs::read_to_string(index_path).ok()?;
    let index_data: Value = serde_json::from_str(&text).ok()?;
    let entries = index_data
        .get("entries")
        .cloned()
        .unwrap_or(index_data);
    let entry_values: Vec<Value> = match &entries {
        Value::Object(m) => m.values().cloned().collect(),
        Value::Array(a) => a.clone(),
        _ => return Some(vec![]),
    };
    let mut paths = Vec::new();
    for entry in entry_values {
        if let Some(cp) = entry
            .as_object()
            .and_then(|o| o.get("common_path"))
            .and_then(|v| v.as_str())
        {
            if !cp.is_empty() {
                paths.push(cp.to_string());
            }
        }
    }
    Some(paths)
}

pub fn reconcile_tags(repo_root: &Path) -> Option<Value> {
    let notes = notes_root_path(repo_root);
    let index_path = notes.join("index.json");
    if !index_path.is_file() {
        return None;
    }
    let paths = index_common_paths(&notes, &index_path)?;
    let old = read_registry(&notes);

    let old_keys = old.get("keys").and_then(|v| v.as_object());
    let mut ref_counts: HashMap<String, u32> = HashMap::new();
    for common_path in &paths {
        let ann = read_annotation_object(&notes, common_path);
        let Some(arr) = ann.get("tag_keys").and_then(|v| v.as_array()) else {
            continue;
        };
        for key in arr.iter().filter_map(|v| v.as_str()) {
            if key.is_empty() {
                continue;
            }
            if old_keys.is_some_and(|m| m.contains_key(key)) {
                *ref_counts.entry(key.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut new_keys = Map::new();
    for (key, count) in &ref_counts {
        let value = old_keys
            .and_then(|m| m.get(key))
            .and_then(|e| e.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        new_keys.insert(
            key.clone(),
            json!({ "value": value, "refs": *count }),
        );
    }

    let new_registry = json!({ "keys": new_keys });

    for common_path in &paths {
        let ann = read_annotation_object(&notes, common_path);
        let Value::Object(mut map) = ann else {
            continue;
        };
        let Some(arr) = map.get("tag_keys").and_then(|v| v.as_array()) else {
            continue;
        };
        let valid: Vec<Value> = arr
            .iter()
            .filter(|v| {
                v.as_str()
                    .map(|k| old_keys.is_some_and(|m| m.contains_key(k)))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let changed = valid.len() != arr.len()
            || valid
                .iter()
                .zip(arr.iter())
                .any(|(a, b)| a != b);
        if !changed {
            continue;
        }
        if valid.is_empty() {
            map.remove("tag_keys");
        } else {
            map.insert("tag_keys".into(), Value::Array(valid));
        }
        let Some(target) = annotation_json_path(&notes, common_path) else {
            return Some(json!({ "error": "Invalid common_path", "_status": 400 }));
        };
        if let Err(e) = atomic_json::write_json(&target, &Value::Object(map)) {
            return Some(json!({ "error": e, "_status": 500 }));
        }
    }

    save_registry(&notes, &new_registry)
}

#[cfg(test)]
#[path = "../unit-tests/services/tags_registry.rs"]
mod tests;
