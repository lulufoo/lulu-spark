//! Atomic JSON write aligned with `server.py` `_write_annotation`.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde_json::Value;

/// Remove empty object layers (same as Python loop before persist).
fn strip_empty_layer_objects(map: &mut serde_json::Map<String, Value>) {
    map.retain(|_, v| !matches!(v, Value::Object(o) if o.is_empty()));
}

/// Pretty JSON with 2-space indent (aligned with Python `json.dumps(..., indent=2)`).
fn json_to_py_style(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map_err(|e| e.to_string())
        .map(|s| s.replace("\n    ", "\n  "))
}

/// Atomic write: `.json.tmp` → rename; empty object after cleanup deletes the file.
pub fn write_json(path: &Path, data: &Value) -> Result<(), String> {
    let mut data = data.clone();
    if let Value::Object(ref mut map) = data {
        strip_empty_layer_objects(map);
        if map.is_empty() {
            if path.exists() {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = json_to_py_style(&data)?;
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn write_json_round_trip_deep_equal() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("ann.json");
        let data = json!({
            "done": true,
            "raw": { "comments": [{ "id": "a", "text": "hi" }] }
        });
        write_json(&path, &data).expect("write");
        let read: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(read, data);
        let pretty = fs::read_to_string(&path).expect("read");
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("done"));
    }

    #[test]
    fn write_json_empty_object_deletes_file() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("ann.json");
        fs::write(&path, r#"{"done":true}"#).expect("seed");
        write_json(&path, &json!({})).expect("write");
        assert!(!path.exists());
    }

    #[test]
    fn write_json_strips_empty_layer_dicts_then_deletes() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("ann.json");
        fs::write(&path, r#"{"raw":{}}"#).expect("seed");
        write_json(&path, &json!({ "raw": {}, "digest": {} })).expect("write");
        assert!(!path.exists());
    }
}
