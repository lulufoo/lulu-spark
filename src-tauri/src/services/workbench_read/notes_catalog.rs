use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::meili_env::notes_root_path;
use crate::integrations::search::MeiliBackend;

use super::notes::{get_notes_file, load_notes_index_entries};

pub const NOTES_SEARCH_DEFAULT_LIMIT: u32 = 5;
const NOTES_SEARCH_MAX_LIMIT: u32 = 50;

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

/// Locate the raw note file. Success is `{id, ok, path}` only — no body text.
pub fn get_note_path_by_id(repo_root: &Path, id: &str) -> Value {
    let id = id.trim();
    if id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    if !is_valid_entry_id(id) {
        return json!({ "id": id, "ok": false, "error": "Invalid id" });
    }
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };
    let Some(entry) = entries.get(id) else {
        return json!({ "id": id, "ok": false, "error": "Entry not found" });
    };
    let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
        return json!({ "id": id, "ok": false, "error": "Missing common_path" });
    };
    let common_path = common_path.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "id": id, "ok": false, "error": "Invalid path" });
    }
    let notes = notes_root_path(repo_root);
    let target = notes.join("raw").join(common_path);
    let Ok(notes_canon) = notes.canonicalize() else {
        return json!({ "id": id, "ok": false, "error": "Notes root missing" });
    };
    let target_canon = target.canonicalize().unwrap_or(target);
    let prefix = format!("{}{}", notes_canon.to_string_lossy(), std::path::MAIN_SEPARATOR);
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return json!({ "id": id, "ok": false, "error": "Path traversal not allowed" });
    }
    if !target_canon.is_file() {
        return json!({ "id": id, "ok": false, "error": format!("File not found: raw/{common_path}") });
    }
    json!({
        "id": id,
        "ok": true,
        "path": target_canon.to_string_lossy(),
    })
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

fn is_safe_search_catalog(catalog: &str) -> bool {
    let raw = catalog.trim();
    !raw.is_empty()
        && raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn parse_notes_search_limit(limit: Option<u32>) -> u32 {
    match limit {
        None | Some(0) => NOTES_SEARCH_DEFAULT_LIMIT,
        Some(n) => n.min(NOTES_SEARCH_MAX_LIMIT),
    }
}

pub(crate) fn notes_raw_search_filter(catalog: Option<&str>) -> Result<String, Value> {
    match catalog.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(r#"layer = "raw""#.to_string()),
        Some(catalog) if is_safe_search_catalog(catalog) => Ok(format!(
            r#"layer = "raw" AND common_path STARTS WITH "{catalog}/""#
        )),
        Some(_) => Err(json!({ "error": "Invalid catalog", "_status": 400 })),
    }
}

fn snippet_from_hit(hit: &Value) -> String {
    hit.pointer("/_formatted/body")
        .and_then(|v| v.as_str())
        .or_else(|| hit.get("body").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string()
}

fn index_by_common_path(entries: &Map<String, Value>) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    for (id, entry) in entries {
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
            .unwrap_or("")
            .to_string();
        map.entry(common_path.to_string())
            .or_insert((id.clone(), created_at));
    }
    map
}

pub(crate) fn project_raw_search_hits(
    hits: &[Value],
    path_index: &HashMap<String, (String, String)>,
) -> Vec<Value> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for hit in hits {
        if hit.get("layer").and_then(|v| v.as_str()) != Some("raw") {
            continue;
        }
        let Some(common_path) = hit.get("common_path").and_then(|v| v.as_str()) else {
            continue;
        };
        if !seen.insert(common_path.to_string()) {
            continue;
        }
        let Some((note_id, created_at)) = path_index.get(common_path) else {
            continue;
        };
        items.push(json!({
            "note_id": note_id,
            "catalog": catalog_from_common_path(common_path),
            "title": hit.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "created_at": created_at,
            "matches": [{ "snippet": snippet_from_hit(hit) }],
        }));
    }
    items
}

/// Search notes in the workbench Meili index. Raw layer only; no digest bodies.
pub fn search_notes(
    repo_root: &Path,
    q: &str,
    catalog: Option<&str>,
    limit: Option<u32>,
) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "Missing q", "_status": 400 });
    }
    let filter = match notes_raw_search_filter(catalog) {
        Ok(f) => f,
        Err(e) => return e,
    };
    let entries = match load_notes_index_entries(repo_root) {
        Ok(e) => e,
        Err(v) => return v,
    };
    let limit = parse_notes_search_limit(limit);
    let result = MeiliBackend::new(repo_root).search_filtered(
        "workbench",
        q,
        Some(limit),
        Some(&filter),
    );
    if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        let status = if err == "q parameter required" { 400 } else { 503 };
        return json!({ "error": err, "_status": status });
    }
    let hits = result
        .get("hits")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    json!({ "items": project_raw_search_hits(&hits, &index_by_common_path(&entries)) })
}
