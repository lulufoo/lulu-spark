//! Knowledge MCP read surface (categories, repos, search, path-by-id).

use std::path::Path;

use serde_json::{json, Value};

use crate::config::meili_env::knowledge_root_string;
use crate::integrations::meilisearch;
use crate::repositories::knowledge::kb_safe_path;
use crate::services::sediment_kb;

use super::doc_map;

fn snippet_from_hit(hit: &Value) -> String {
    hit.pointer("/_formatted/body")
        .and_then(|v| v.as_str())
        .or_else(|| hit.get("body").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string()
}

pub fn list_knowledge_categories_value() -> Value {
    match sediment_kb::load_categories() {
        Ok(file) => json!(file
            .categories
            .into_iter()
            .map(|c| json!({ "id": c.id, "name": c.name }))
            .collect::<Vec<_>>()),
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

pub fn list_knowledge_repos_value() -> Value {
    match sediment_kb::list_repos_for_topics() {
        Ok(rows) => json!(rows
            .into_iter()
            .map(|r| json!({
                "full_name": r.repo,
                "description": r.description,
                "category_id": r.category_id,
                "category_name": r.category_name,
            }))
            .collect::<Vec<_>>()),
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

pub fn search_knowledge_mcp(repo_root: &Path, q: &str, limit: Option<u32>) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "Missing q", "_status": 400 });
    }
    let result = meilisearch::search_json(repo_root, "knowledge", q, limit);
    if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        let status = if err == "q parameter required" { 400 } else { 503 };
        return json!({ "error": err, "_status": status });
    }
    let hits = result
        .get("hits")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let kb_root = std::path::PathBuf::from(knowledge_root_string(repo_root));
    let mut items = Vec::new();
    for hit in hits {
        let Some(repo) = hit.get("repo").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(rel) = hit.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        let Ok(abs) = kb_safe_path(&kb_root, repo, rel) else {
            continue;
        };
        if !abs.is_file() {
            continue;
        }
        let id = match doc_map::remember_path(&abs) {
            Ok(id) => id,
            Err(e) => return json!({ "error": e, "_status": 500 }),
        };
        let title = hit
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        items.push(json!({
            "id": id,
            "title": title,
            "snippet": snippet_from_hit(&hit),
        }));
    }
    json!({ "items": items })
}

/// Resolve a search-issued id to a local file path. Does not return file body.
pub fn get_knowledge_path_by_id(id: &str) -> Value {
    let id = id.trim();
    if id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    let path = match doc_map::lookup_path(id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return json!({
                "id": id,
                "ok": false,
                "error": "Unknown document id",
            });
        }
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    if !path.is_file() {
        return json!({
            "id": id,
            "ok": false,
            "error": "File not found",
        });
    }
    json!({
        "id": id,
        "ok": true,
        "path": path.to_string_lossy(),
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/mcp.rs"]
mod tests;
