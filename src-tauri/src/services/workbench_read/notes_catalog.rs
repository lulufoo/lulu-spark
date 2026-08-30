use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use super::notes::{get_notes_file, load_notes_index_entries};

pub const RAW_CONTENT_MAX_BYTES: usize = 10 * 1024;

fn catalog_from_common_path(common_path: &str) -> &str {
    common_path.split('/').next().unwrap_or(common_path)
}

fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_catalog_slug(catalog: &str) -> bool {
    let raw = catalog.trim();
    !raw.is_empty() && !raw.contains("..") && !raw.contains('/') && !raw.contains('\\')
}

fn entry_has_digest_layer(entry: &Value) -> bool {
    entry
        .get("layers")
        .and_then(|v| v.as_array())
        .is_some_and(|layers| layers.iter().any(|l| l.as_str() == Some("digest")))
}

pub(crate) fn truncate_utf8(text: &str, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text.to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

/// All project catalogs with the newest note pointer (no digest body).
pub fn list_all_notes_catalogs(repo_root: &Path) -> Value {
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };

    let mut best_by_catalog: HashMap<String, (String, String)> = HashMap::new();
    for (id, entry) in &entries {
        if !is_valid_entry_id(id) {
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
        let catalog = catalog_from_common_path(common_path).to_string();
        match best_by_catalog.get(&catalog) {
            None => {
                best_by_catalog.insert(catalog, (id.clone(), created_at.to_string()));
            }
            Some((_, prev_created)) if created_at > prev_created.as_str() => {
                best_by_catalog.insert(catalog, (id.clone(), created_at.to_string()));
            }
            _ => {}
        }
    }

    let mut items: Vec<Value> = best_by_catalog
        .into_iter()
        .map(|(catalog, (note_id, created_at))| {
            json!({
                "catalog": catalog,
                "note_id": note_id,
                "created_at": created_at,
            })
        })
        .collect();
    items.sort_by(|a, b| {
        a.get("catalog")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("catalog").and_then(|v| v.as_str()).unwrap_or(""))
    });
    json!({ "items": items })
}

/// Newest digest-bearing note per catalog, with digest body. One MCP round-trip.
pub fn get_latest_digest_per_catalog(repo_root: &Path) -> Value {
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };

    let mut best_by_catalog: HashMap<String, (String, String, String)> = HashMap::new();
    for (id, entry) in &entries {
        if !is_valid_entry_id(id) || !entry_has_digest_layer(entry) {
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
        let catalog = catalog_from_common_path(common_path).to_string();
        match best_by_catalog.get(&catalog) {
            None => {
                best_by_catalog.insert(
                    catalog,
                    (id.clone(), created_at.to_string(), common_path.to_string()),
                );
            }
            Some((_, prev_created, _)) if created_at > prev_created.as_str() => {
                best_by_catalog.insert(
                    catalog,
                    (id.clone(), created_at.to_string(), common_path.to_string()),
                );
            }
            _ => {}
        }
    }

    let mut items: Vec<Value> = best_by_catalog
        .into_iter()
        .map(|(catalog, (note_id, created_at, common_path))| {
            let file_value = get_notes_file(repo_root, "digest", &common_path);
            if let Some(status) = file_value.get("_status").and_then(|v| v.as_u64()) {
                let msg = file_value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("read failed");
                return json!({
                    "catalog": catalog,
                    "note_id": note_id,
                    "created_at": created_at,
                    "ok": false,
                    "error": format!("HTTP {status}: {msg}"),
                });
            }
            json!({
                "catalog": catalog,
                "note_id": note_id,
                "created_at": created_at,
                "ok": true,
                "content": file_value.get("content").and_then(|v| v.as_str()).unwrap_or(""),
            })
        })
        .collect();
    items.sort_by(|a, b| {
        a.get("catalog")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("catalog").and_then(|v| v.as_str()).unwrap_or(""))
    });
    json!({ "items": items })
}

/// All note ids in one project catalog, newest `created_at` first.
pub fn list_notes_by_catalog(repo_root: &Path, catalog: &str) -> Value {
    if !is_valid_catalog_slug(catalog) {
        return json!({ "error": "Invalid catalog", "_status": 400 });
    }
    let want = catalog.trim();
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };

    let mut rows: Vec<(String, String)> = Vec::new();
    for (id, entry) in &entries {
        if !is_valid_entry_id(id) {
            continue;
        }
        let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
            continue;
        };
        if catalog_from_common_path(common_path) != want {
            continue;
        }
        let created_at = entry
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        rows.push((id.clone(), created_at));
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    json!({ "ids": rows.into_iter().map(|(id, _)| id).collect::<Vec<_>>() })
}

pub fn get_note_digest_by_id(repo_root: &Path, id: &str) -> Value {
    read_note_layer(repo_root, id, "digest", false)
}

pub fn get_note_content_by_id(repo_root: &Path, id: &str) -> Value {
    read_note_layer(repo_root, id, "raw", true)
}

fn read_note_layer(repo_root: &Path, id: &str, layer: &str, truncate_raw: bool) -> Value {
    let id = id.trim();
    if id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    if !is_valid_entry_id(id) {
        return json!({
            "id": id,
            "ok": false,
            "error": "Invalid id",
        });
    }
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };
    let Some(entry) = entries.get(id) else {
        return json!({
            "id": id,
            "ok": false,
            "error": "Entry not found",
        });
    };
    let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
        return json!({
            "id": id,
            "ok": false,
            "error": "Missing common_path",
        });
    };
    let file_value = get_notes_file(repo_root, layer, common_path);
    if let Some(status) = file_value.get("_status").and_then(|v| v.as_u64()) {
        let msg = file_value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("read failed");
        return json!({
            "id": id,
            "ok": false,
            "error": format!("HTTP {status}: {msg}"),
        });
    }
    let raw = file_value
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if truncate_raw {
        let (content, truncated) = truncate_utf8(raw, RAW_CONTENT_MAX_BYTES);
        return json!({
            "id": id,
            "ok": true,
            "content": content,
            "truncated": truncated,
        });
    }
    json!({
        "id": id,
        "ok": true,
        "content": raw,
    })
}
