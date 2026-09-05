//! Knowledge MCP read surface (categories, repos, search, path-by-id).

use std::path::Path;

use serde_json::{json, Value};

use crate::config::paths;
use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::kb_safe_path;
use crate::services::keyword_index::{index_exists, search_in_cache, SearchFilter};
use crate::services::sediment_kb;

use super::doc_map;

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
    let cache = match paths::cache_dir() {
        Ok(p) => p,
        Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
    };
    if !index_exists(&cache) {
        return json!({ "error": "not_indexed", "_status": 503 });
    }
    let filter = SearchFilter {
        category: Some("knowledge".into()),
        ..SearchFilter::default()
    };
    let hits = match search_in_cache(&cache, q, &filter, limit.unwrap_or(10).min(50)) {
        Ok(h) => h,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let kb_root = std::path::PathBuf::from(knowledge_root_string(repo_root));
    let mut items = Vec::new();
    for hit in hits {
        let abs = std::path::PathBuf::from(&hit.path);
        let abs = if abs.is_file() {
            abs
        } else {
            match kb_safe_path(&kb_root, &hit.repo, &hit.common_path) {
                Ok(p) if p.is_file() => p,
                _ => continue,
            }
        };
        let id = match doc_map::remember_path(&abs) {
            Ok(id) => id,
            Err(e) => return json!({ "error": e, "_status": 500 }),
        };
        items.push(json!({
            "id": id,
            "title": hit.title,
            "snippet": hit.snippet,
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
