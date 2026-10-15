//! Knowledge MCP read surface (categories, repos, search, path-by-id).

use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::config::paths;
use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::kb_safe_path;
use crate::services::search_index::{index_exists, search_in_cache, SearchFilter};
use crate::services::sediment_kb;
use crate::services::spark_data::{self, get_knowledge_doc, register_knowledge_doc};

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
        if !abs.is_file() {
            continue;
        }
        items.push(json!({
            "id": hit.doc_id,
            "title": hit.title,
            "snippet": hit.snippet,
        }));
    }
    json!({ "items": items })
}

/// `{root}/<repo>/<a>/<b>.md` → `("<repo>", "a/b.md")`. A file needs a repo directory.
fn library_location(root: &Path, file: &Path) -> Option<(String, String)> {
    let rel = file.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(name) => parts.push(name.to_str()?.to_string()),
            _ => return None,
        }
    }
    if parts.len() < 2 {
        return None;
    }
    let repo = parts.remove(0);
    Some((repo, parts.join("/")))
}

/// Look up, or register, the `doc_id` of an absolute knowledge file.
pub fn remember_knowledge_doc_value(path: &str) -> Value {
    let path = path.trim();
    if path.is_empty() {
        return json!({ "error": "Missing path", "_status": 400 });
    }
    let root = match paths::knowledge_root() {
        Ok(root) => root.canonicalize().unwrap_or(root),
        Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
    };
    let file = Path::new(path);
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let Some((repo, rel)) = library_location(&root, &file) else {
        return json!({ "error": "Not in the knowledge library", "_status": 400 });
    };
    if !file.is_file() {
        return json!({ "error": "File not found", "_status": 404 });
    }
    let registered = spark_data::open().and_then(|conn| register_knowledge_doc(&conn, &repo, &rel));
    match registered {
        Ok(id) => json!({ "ok": true, "id": id }),
        Err(e) => json!({ "error": e, "_status": 500 }),
    }
}

/// Resolve a `doc_id` to `(repo, relative path)` plus the file on disk. Does not
/// return the file body.
pub(super) struct LocatedDoc {
    pub repo: String,
    pub path: String,
    pub file: PathBuf,
}

pub(super) fn locate_knowledge_doc(id: &str) -> Result<LocatedDoc, Value> {
    let id = id.trim();
    if id.is_empty() {
        return Err(json!({ "error": "Missing id", "_status": 400 }));
    }
    let row = spark_data::open()
        .and_then(|conn| get_knowledge_doc(&conn, id))
        .map_err(|e| json!({ "error": e, "_status": 500 }))?;
    let Some(row) = row else {
        return Err(json!({ "id": id, "ok": false, "error": "Unknown document id" }));
    };
    let root = paths::knowledge_root().map_err(|e| json!({ "error": format!("{e:?}"), "_status": 500 }))?;
    let file = kb_safe_path(&root, &row.repo, &row.path)
        .ok()
        .filter(|p| p.is_file());
    match file {
        Some(file) => Ok(LocatedDoc { repo: row.repo, path: row.path, file }),
        None => Err(json!({ "id": id, "ok": false, "error": "File not found" })),
    }
}

/// Resolve a `doc_id` to a local file path. Does not return file body.
pub fn get_knowledge_path_by_id(id: &str) -> Value {
    match locate_knowledge_doc(id) {
        Ok(doc) => json!({ "id": id.trim(), "ok": true, "path": doc.file.to_string_lossy() }),
        Err(reply) => reply,
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/mcp.rs"]
mod tests;
