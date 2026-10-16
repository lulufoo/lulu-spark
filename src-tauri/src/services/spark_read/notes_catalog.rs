use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::paths;
use crate::services::search_index::{index_exists, search_in_cache, SearchFilter};

use super::notes::{get_notes_file, load_notes_index_entries};

pub const NOTES_SEARCH_DEFAULT_LIMIT: u32 = 5;
const NOTES_SEARCH_MAX_LIMIT: u32 = 50;

pub const RAW_CONTENT_MAX_BYTES: usize = 10 * 1024;

fn entry_folder(entry: &Value) -> Option<&str> {
    entry.get("folder").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

pub(super) fn is_valid_entry_id(id: &str) -> bool {
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

/// Newest digest-bearing note per catalog, with digest body. One MCP round-trip.
pub fn get_latest_digest_per_catalog(repo_root: &Path) -> Value {
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };

    let mut best_by_catalog: HashMap<String, (String, String)> = HashMap::new();
    for (id, entry) in &entries {
        if !is_valid_entry_id(id) || !entry_has_digest_layer(entry) {
            continue;
        }
        let Some(catalog) = entry_folder(entry) else {
            continue;
        };
        let created_at = entry
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match best_by_catalog.get(catalog) {
            None => {
                best_by_catalog.insert(catalog.to_string(), (id.clone(), created_at.to_string()));
            }
            Some((_, prev_created)) if created_at > prev_created.as_str() => {
                best_by_catalog.insert(catalog.to_string(), (id.clone(), created_at.to_string()));
            }
            _ => {}
        }
    }

    let mut items: Vec<Value> = best_by_catalog
        .into_iter()
        .map(|(catalog, (note_id, created_at))| {
            let file_value = get_notes_file(repo_root, "digest", &note_id, None);
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
        if entry_folder(entry) != Some(want) {
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

pub fn get_note_digest(repo_root: &Path, id: &str) -> Value {
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
    let file_value = get_notes_file(repo_root, layer, id, None);
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

fn parse_notes_search_limit(limit: Option<u32>) -> u32 {
    match limit {
        None | Some(0) => NOTES_SEARCH_DEFAULT_LIMIT,
        Some(n) => n.min(NOTES_SEARCH_MAX_LIMIT),
    }
}

fn snippet_from_hit(hit: &Value) -> String {
    hit.pointer("/_formatted/body")
        .and_then(|v| v.as_str())
        .or_else(|| hit.get("body").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string()
}

pub(crate) fn project_raw_search_hits(
    hits: &[Value],
    entries: &Map<String, Value>,
) -> Vec<Value> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for hit in hits {
        if hit.get("layer").and_then(|v| v.as_str()) != Some("raw") {
            continue;
        }
        let Some(note_id) = hit.get("note_id").and_then(|v| v.as_str()) else {
            continue;
        };
        if !seen.insert(note_id.to_string()) {
            continue;
        }
        let Some(entry) = entries.get(note_id) else {
            continue;
        };
        items.push(json!({
            "note_id": note_id,
            "catalog": entry_folder(entry).unwrap_or(""),
            "title": hit.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "created_at": entry.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
            "matches": [{ "snippet": snippet_from_hit(hit) }],
        }));
    }
    items
}

/// Search notes in the search index. Raw layer only; no digest bodies.
pub fn search_notes(
    repo_root: &Path,
    q: &str,
    limit: Option<u32>,
) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "Missing q", "_status": 400 });
    }
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };
    let limit = parse_notes_search_limit(limit);
    let cache = match paths::cache_dir() {
        Ok(p) => p,
        Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
    };
    if !index_exists(&cache) {
        return json!({ "error": "not_indexed", "_status": 503 });
    }
    let fts_filter = SearchFilter {
        workspace: Some(crate::services::spark_data::DEFAULT_WORKSPACE_ID.into()),
        layer: Some("raw".into()),
        ..SearchFilter::default()
    };
    let hits = match search_in_cache(&cache, q, &fts_filter, limit) {
        Ok(h) => h,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let projected: Vec<Value> = hits
        .into_iter()
        .map(|h| {
            json!({
                "layer": "raw",
                "note_id": h.doc_id,
                "title": h.title,
                "_formatted": { "body": h.snippet },
            })
        })
        .collect();
    json!({ "items": project_raw_search_hits(&projected, &entries) })
}
