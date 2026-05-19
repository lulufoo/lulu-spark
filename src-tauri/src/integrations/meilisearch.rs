use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeiliConfig {
    pub url: String,
    pub master_key: String,
}

pub fn load_meili_config(repo_root: &Path) -> MeiliConfig {
    let mut url = "http://localhost:7700".to_string();
    let mut master_key = String::new();
    let env_path = repo_root.join("meili.env");
    if let Ok(text) = std::fs::read_to_string(&env_path) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "MEILI_URL" => url = v.trim().to_string(),
                "MEILI_MASTER_KEY" => master_key = v.trim().to_string(),
                _ => {}
            }
        }
    }
    MeiliConfig { url, master_key }
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

pub fn search_json(index_uid: &str, q: &str, limit: Option<u32>) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "q parameter required" });
    }
    let limit = parse_limit(limit);
    let repo_root = crate::config::paths::repo_root().unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    });
    let cfg = load_meili_config(&repo_root);
    let body = build_search_body(q, limit);
    let result = meili_post(&cfg, index_uid, &body);
    map_search_result(result)
}

fn meili_post(config: &MeiliConfig, index_uid: &str, body: &Value) -> Option<Value> {
    let url = format!(
        "{}{}",
        config.url.trim_end_matches('/'),
        search_path(index_uid)
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .no_proxy()
        .build()
        .ok()?;
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.master_key))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .ok()?;
    let status = resp.status();
    let text = resp.text().ok()?;
    let parsed: Value = serde_json::from_str(&text).ok()?;
    if status.is_success() {
        Some(parsed)
    } else {
        Some(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let v = search_json("knowledge", "  ", None);
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
    fn load_meili_config_reads_repo_meili_env() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let cfg = load_meili_config(&root);
        assert_eq!(cfg.url, "http://localhost:7700");
        assert!(!cfg.master_key.is_empty());
    }
}
