//! Tag attach / detach / update_value on the `note` and `tag` tables.

use std::path::Path;

use serde_json::{json, Value};

use crate::services::id::random_hex12;
use crate::services::note_edit::edit_note;
use crate::services::spark_data::{
    self, find_tag_key_by_value, has_tag, insert_tag, update_tag_value,
};

pub const TAG_VALUE_MAX_LEN: usize = 64;

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

fn internal(error: impl ToString) -> Value {
    json!({ "error": error.to_string(), "_status": 500 })
}

fn note_keys(tags: &Value) -> Vec<String> {
    tags.as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn set_note_keys(tags: &mut Value, mut keys: Vec<String>) {
    keys.sort();
    keys.dedup();
    *tags = json!(keys);
}

/// Add `key` to the note's tags; a key it already has is a no-op.
fn attach_key(note_tags: &mut Value, key: &str) -> Value {
    let mut keys = note_keys(note_tags);
    if keys.iter().any(|k| k == key) {
        return json!({ "ok": true, "idempotent": true, "key": key });
    }
    keys.push(key.to_string());
    set_note_keys(note_tags, keys);
    json!({ "ok": true, "key": key })
}

pub fn tag_attach(_repo_root: &Path, id: &str, payload: &Value) -> Value {
    let key_opt = payload.get("key").and_then(|v| v.as_str()).map(str::trim);
    let value_opt = payload.get("value").and_then(|v| v.as_str());
    if let Some(key) = key_opt {
        if key.is_empty() {
            return json!({ "error": "Invalid key", "_status": 400 });
        }
        return edit_note(id, |conn, note| {
            if !has_tag(conn, key).map_err(internal)? {
                return Err(json!({ "error": "Unknown tag key", "_status": 400 }));
            }
            Ok(attach_key(&mut note.tags, key))
        });
    }
    let value = match value_opt {
        Some(v) => match validate_value(v) {
            Ok(s) => s,
            Err(e) => return e,
        },
        None => return json!({ "error": "value required when key omitted", "_status": 400 }),
    };
    edit_note(id, |conn, note| {
        let key = match find_tag_key_by_value(conn, &value).map_err(internal)? {
            Some(existing) => existing,
            None => {
                let new_key = random_hex12();
                insert_tag(conn, &new_key, &value).map_err(internal)?;
                new_key
            }
        };
        Ok(attach_key(&mut note.tags, &key))
    })
}

pub fn tag_detach(_repo_root: &Path, id: &str, key: &str) -> Value {
    let key = key.trim();
    if key.is_empty() {
        return json!({ "error": "Invalid key", "_status": 400 });
    }
    // A tag no note references any more is dropped by `edit_note` once the row is written.
    edit_note(id, |_, note| {
        let mut keys = note_keys(&note.tags);
        keys.retain(|k| k != key);
        set_note_keys(&mut note.tags, keys);
        Ok(json!({ "ok": true }))
    })
}

pub fn tag_update_value(_repo_root: &Path, key: &str, value: &str) -> Value {
    let key = key.trim();
    if key.is_empty() {
        return json!({ "error": "Invalid key", "_status": 400 });
    }
    let value = match validate_value(value) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let renamed = spark_data::open().and_then(|conn| update_tag_value(&conn, key, &value));
    match renamed {
        Ok(true) => json!({ "ok": true }),
        Ok(false) => json!({ "error": "Unknown tag key", "_status": 400 }),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/tag_write.rs"]
mod tests;
