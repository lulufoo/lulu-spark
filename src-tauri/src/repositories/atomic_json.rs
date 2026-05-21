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
#[path = "../unit-tests/repositories/atomic_json.rs"]
mod tests;
