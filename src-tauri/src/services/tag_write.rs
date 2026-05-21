//! Tag attach / detach / update_value — coordinates annotation + registry.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::repositories::annotation_paths::annotation_json_path;
use crate::repositories::atomic_json;
use crate::services::annotation::read_annotation_object;
use crate::services::id::random_hex12;
use crate::services::tags_registry::{adjust_refs, read_registry, save_registry};

pub const TAG_VALUE_MAX_LEN: usize = 64;

fn invalid_common_path() -> Value {
    json!({ "error": "Invalid common_path", "_status": 400 })
}

fn corpus_and_path(repo_root: &Path, common_path: &str) -> Result<(PathBuf, PathBuf), Value> {
    let cp = common_path.trim();
    if cp.is_empty() || cp.contains("..") {
        return Err(invalid_common_path());
    }
    let corpus = workbench_knowledge_root_path(repo_root);
    let Some(target) = annotation_json_path(&corpus, cp) else {
        return Err(invalid_common_path());
    };
    Ok((corpus, target))
}

fn validate_value(value: &str) -> Result<String, Value> {
    let v = value.trim();
    if v.is_empty() {
        return Err(json!({ "error": "value must not be empty", "_status": 400 }));
    }
    if v.len() > TAG_VALUE_MAX_LEN {
        return Err(json!({
            "error": format!("value length exceeds {TAG_VALUE_MAX_LEN}"),
            "_status": 400
        }));
    }
    Ok(v.to_string())
}

fn persist_annotation_file(target: &Path, ann: &Value) -> Option<Value> {
    if let Some(parent) = target.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Some(json!({ "error": e.to_string(), "_status": 500 }));
        }
    }
    atomic_json::write_json(target, ann)
        .err()
        .map(|e| json!({ "error": e, "_status": 500 }))
}

fn dedup_tag_keys(keys: &mut Vec<String>) {
    keys.sort();
    keys.dedup();
}

fn tag_keys_vec(ann: &Value) -> Vec<String> {
    ann.get("tag_keys")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub fn tag_attach(repo_root: &Path, common_path: &str, payload: &Value) -> Value {
    let Ok((corpus, target)) = corpus_and_path(repo_root, common_path) else {
        return invalid_common_path();
    };
    let cp = common_path.trim();

    let key_opt = payload.get("key").and_then(|v| v.as_str()).map(str::trim);
    let value_opt = payload.get("value").and_then(|v| v.as_str());

    let mut registry = read_registry(&corpus);
    let registry_snapshot = registry.clone();

    let mut ann = read_annotation_object(&corpus, cp);
    let mut keys = tag_keys_vec(&ann);
    let Value::Object(ref mut ann_map) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };

    if let Some(key) = key_opt {
        if key.is_empty() {
            return json!({ "error": "Invalid key", "_status": 400 });
        }
        let reg_keys = registry.get("keys").and_then(|v| v.as_object());
        if !reg_keys.is_some_and(|m| m.contains_key(key)) {
            return json!({ "error": "Unknown tag key", "_status": 400 });
        }
        if keys.iter().any(|k| k == key) {
            return json!({ "ok": true, "idempotent": true, "key": key });
        }
        keys.push(key.to_string());
        adjust_refs(&mut registry, &[key.to_string()], 1);
        dedup_tag_keys(&mut keys);
        ann_map.insert("tag_keys".into(), json!(keys));
        if let Some(err) = save_registry(&corpus, &registry) {
            return err;
        }
        if let Some(err) = persist_annotation_file(&target, &ann) {
            let _ = save_registry(&corpus, &registry_snapshot);
            return err;
        }
        return json!({ "ok": true, "key": key });
    }

    let value = match value_opt {
        Some(v) => match validate_value(v) {
            Ok(s) => s,
            Err(e) => return e,
        },
        None => {
            return json!({ "error": "value required when key omitted", "_status": 400 });
        }
    };

    let new_key = random_hex12();
    keys.push(new_key.clone());
    dedup_tag_keys(&mut keys);
    ann_map.insert("tag_keys".into(), json!(keys));

    let reg_keys = registry
        .get_mut("keys")
        .and_then(|v| v.as_object_mut())
        .expect("keys object");
    reg_keys.insert(new_key.clone(), json!({ "value": value, "refs": 1 }));

    if let Some(err) = save_registry(&corpus, &registry) {
        return err;
    }
    if let Some(err) = persist_annotation_file(&target, &ann) {
        let _ = save_registry(&corpus, &registry_snapshot);
        return err;
    }
    json!({ "ok": true, "key": new_key })
}

pub fn tag_detach(repo_root: &Path, common_path: &str, key: &str) -> Value {
    let Ok((corpus, target)) = corpus_and_path(repo_root, common_path) else {
        return invalid_common_path();
    };
    let cp = common_path.trim();
    let key = key.trim();
    if key.is_empty() {
        return json!({ "error": "Invalid key", "_status": 400 });
    }

    let mut registry = read_registry(&corpus);
    let registry_snapshot = registry.clone();

    let mut ann = read_annotation_object(&corpus, cp);
    let mut keys = tag_keys_vec(&ann);
    let Value::Object(ref mut ann_map) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };

    if !keys.iter().any(|k| k == key) {
        return json!({ "ok": true });
    }
    keys.retain(|k| k != key);
    adjust_refs(&mut registry, &[key.to_string()], -1);

    if keys.is_empty() {
        ann_map.remove("tag_keys");
    } else {
        ann_map.insert("tag_keys".into(), json!(keys));
    }

    if let Some(err) = save_registry(&corpus, &registry) {
        return err;
    }
    if let Some(err) = persist_annotation_file(&target, &ann) {
        let _ = save_registry(&corpus, &registry_snapshot);
        return err;
    }
    json!({ "ok": true })
}

pub fn tag_update_value(repo_root: &Path, key: &str, value: &str) -> Value {
    let key = key.trim();
    if key.is_empty() {
        return json!({ "error": "Invalid key", "_status": 400 });
    }
    let value = match validate_value(value) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let corpus = workbench_knowledge_root_path(repo_root);
    let mut registry = read_registry(&corpus);
    let Some(reg_keys) = registry.get_mut("keys").and_then(|v| v.as_object_mut()) else {
        return json!({ "error": "Invalid registry", "_status": 500 });
    };
    let Some(entry) = reg_keys.get_mut(key) else {
        return json!({ "error": "Unknown tag key", "_status": 400 });
    };
    let Some(obj) = entry.as_object_mut() else {
        return json!({ "error": "Invalid registry entry", "_status": 500 });
    };
    obj.insert("value".into(), json!(value));

    if let Some(err) = save_registry(&corpus, &registry) {
        return err;
    }
    json!({ "ok": true })
}

#[cfg(test)]
#[path = "../unit-tests/services/tag_write.rs"]
mod tests;
