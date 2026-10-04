//! Notes store lives at `{spark_root}/notes/`.
//! One-time move from the former notes-at-root layout.

use std::fs;
use std::path::Path;

use serde_json::Value;

const NOTE_ITEMS: &[&str] = &["raw", "digest", "annotations", "tags", "index.json"];
const DEAD_LAYERS: &[&str] = &["distilled", "diagnose", "trace"];

/// Ensure `{wb}/notes/index.json` exists, moving note items off the spark root if needed.
pub fn ensure_notes_layout(wb: &Path) -> Result<(), String> {
    let notes = wb.join("notes");
    let new_index = notes.join("index.json");
    if new_index.is_file() {
        return Ok(());
    }
    let old_index = wb.join("index.json");
    if !old_index.is_file() {
        return Ok(());
    }
    fs::create_dir_all(&notes).map_err(|e| e.to_string())?;
    for name in NOTE_ITEMS {
        let src = wb.join(name);
        if src.exists() {
            let dest = notes.join(name);
            if !dest.exists() {
                fs::rename(&src, &dest).map_err(|e| e.to_string())?;
            }
        }
    }
    remove_dead_dirs(wb);
    remove_dead_dirs(&notes);
    if new_index.is_file() {
        strip_dead_layers(&new_index)?;
    }
    Ok(())
}

fn remove_dead_dirs(root: &Path) {
    for name in DEAD_LAYERS {
        let p = root.join(name);
        if p.is_dir() {
            let _ = fs::remove_dir_all(p);
        }
    }
}

fn strip_dead_layers(index_path: &Path) -> Result<(), String> {
    let text = fs::read_to_string(index_path).map_err(|e| e.to_string())?;
    let mut data: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let Some(entries) = data.get_mut("entries").and_then(|v| v.as_object_mut()) else {
        return Ok(());
    };
    let mut changed = false;
    for entry in entries.values_mut() {
        let Some(layers) = entry.get_mut("layers").and_then(|v| v.as_array_mut()) else {
            continue;
        };
        let next: Vec<Value> = layers
            .iter()
            .filter(|l| matches!(l.as_str(), Some("raw") | Some("digest")))
            .cloned()
            .collect();
        if next.len() != layers.len() {
            *layers = next;
            changed = true;
        }
    }
    if changed {
        let pretty = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        fs::write(index_path, pretty).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/services/notes_layout.rs"]
mod tests;
