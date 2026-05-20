use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use crate::config::meili_env::{meili_master_key, meili_url};
use crate::integrations::search::SearchBackend;

#[derive(Clone)]
pub struct MeiliBackend {
    pub(crate) url: String,
    pub(crate) master_key: String,
}

impl MeiliBackend {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            url: meili_url(repo_root),
            master_key: meili_master_key(repo_root),
        }
    }

    fn meili_post(&self, index_uid: &str, body: &Value) -> Option<Value> {
        let url = format!(
            "{}{}",
            self.url.trim_end_matches('/'),
            search_path(index_uid)
        );
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .build()
            .ok()?;
        let resp = client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.master_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .ok()?;
        let text = resp.text().ok()?;
        serde_json::from_str(&text).ok()
    }
}

impl SearchBackend for MeiliBackend {
    fn health(&self) -> bool {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .no_proxy()
            .build();
        let Ok(client) = client else {
            return false;
        };
        let url = format!("{}/health", self.url.trim_end_matches('/'));
        client.get(url).send().map(|r| r.status().is_success()).unwrap_or(false)
    }

    fn search(&self, index_uid: &str, q: &str, limit: Option<u32>) -> Value {
        let q = q.trim();
        if q.is_empty() {
            return json!({ "error": "q parameter required" });
        }
        let limit = parse_limit(limit);
        let body = build_search_body(q, limit);
        let result = self.meili_post(index_uid, &body);
        map_search_result(result)
    }
}

pub fn parse_limit(limit: Option<u32>) -> u32 {
    limit.unwrap_or(10).min(50)
}

pub fn build_search_body(q: &str, limit: u32) -> Value {
    json!({
        "q": q,
        "limit": limit,
        "attributesToCrop": ["body"],
        "cropLength": 80,
        "attributesToHighlight": ["body"],
    })
}

pub fn search_path(index_uid: &str) -> String {
    format!("/indexes/{index_uid}/search")
}

pub fn documents_path(index_uid: &str) -> String {
    format!("/indexes/{index_uid}/documents")
}

/// Build Meilisearch knowledge document (aligned with `server.py::_meili_upsert_doc`).
pub fn build_knowledge_document(repo: &str, path: &str, content: &str, topic_desc: &str) -> Value {
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let raw_id = format!("{repo_name}__{}", path.replace('/', "__"));
    let doc_id: String = raw_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(511)
        .collect();
    let mut title = String::new();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix('#') {
            let t = rest.trim_start();
            if !t.is_empty() {
                title = t.to_string();
                break;
            }
        }
    }
    if title.is_empty() {
        title = path
            .split('/')
            .next_back()
            .unwrap_or(path)
            .trim_end_matches(".md")
            .to_string();
    }
    let url = format!("https://github.com/{repo}/blob/main/{path}");
    json!({
        "id": doc_id,
        "title": title,
        "body": content,
        "repo": repo,
        "path": path,
        "url": url,
        "topic_desc": topic_desc,
    })
}

impl MeiliBackend {
    /// Best-effort upsert; returns error message string on failure.
    /// Single-document upsert (settle path); batch index build uses `put_documents`.
    pub fn upsert_knowledge_documents(&self, docs: &[Value]) -> Result<(), String> {
        if docs.is_empty() {
            return Ok(());
        }
        let url = format!(
            "{}{}",
            self.url.trim_end_matches('/'),
            documents_path("knowledge")
        );
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .put(url)
            .header("Authorization", format!("Bearer {}", self.master_key))
            .header("Content-Type", "application/json")
            .json(docs)
            .send()
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            Err(if text.trim().is_empty() {
                format!("Meilisearch HTTP {status}")
            } else {
                text
            })
        }
    }
}

/// Map Meilisearch HTTP result to API JSON (aligned with `server.py` search handlers).
pub fn map_search_result(meili_result: Option<Value>) -> Value {
    let Some(result) = meili_result else {
        return json!({ "hits": [], "error": "unavailable" });
    };
    if result.get("code").and_then(|c| c.as_str()) == Some("index_not_found") {
        return json!({ "hits": [], "error": "not_indexed" });
    }
    let hits = result.get("hits").cloned().unwrap_or_else(|| json!([]));
    json!({ "hits": hits })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_search_body_has_required_fields() {
        let body = build_search_body("rust", 10);
        assert_eq!(body["q"], "rust");
        assert_eq!(body["limit"], 10);
        assert_eq!(body["attributesToCrop"], json!(["body"]));
        assert_eq!(body["cropLength"], 80);
        assert_eq!(body["attributesToHighlight"], json!(["body"]));
    }

    #[test]
    fn parse_limit_caps_at_50() {
        assert_eq!(parse_limit(None), 10);
        assert_eq!(parse_limit(Some(100)), 50);
    }

    #[test]
    fn empty_q_returns_400_shape() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let backend = MeiliBackend::new(&root);
        let v = backend.search("knowledge", "  ", None);
        assert_eq!(v, json!({ "error": "q parameter required" }));
    }

    #[test]
    fn map_unavailable_and_not_indexed() {
        assert_eq!(
            map_search_result(None),
            json!({ "hits": [], "error": "unavailable" })
        );
        assert_eq!(
            map_search_result(Some(json!({ "code": "index_not_found" }))),
            json!({ "hits": [], "error": "not_indexed" })
        );
    }

    #[test]
    fn map_success_hits() {
        let v = map_search_result(Some(json!({
            "hits": [{ "id": "a", "title": "t" }]
        })));
        assert_eq!(v["hits"][0]["id"], "a");
        assert!(v.get("error").is_none());
    }

    #[test]
    fn search_path_uses_index_uid() {
        assert_eq!(search_path("knowledge"), "/indexes/knowledge/search");
        assert_eq!(search_path("workbench"), "/indexes/workbench/search");
    }

    #[test]
    fn build_knowledge_document_extracts_title() {
        let doc = build_knowledge_document(
            "o/r",
            "docs/a.md",
            "# Hello\n\nbody",
            "desc",
        );
        assert_eq!(doc["title"], "Hello");
        assert_eq!(doc["repo"], "o/r");
        assert!(doc["id"].as_str().unwrap().starts_with("r__docs__"));
    }
}
