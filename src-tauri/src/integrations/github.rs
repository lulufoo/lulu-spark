//! GitHub REST API (replaces `gh` CLI). PAT from Keychain via `config::secrets`.

use std::time::Duration;

use base64::Engine;
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::Value;

use crate::config::secrets::{self, KEY_GITHUB_TOKEN};

const API_BASE: &str = "https://api.github.com";

#[derive(Debug, Clone)]
pub struct GithubError {
    pub message: String,
    pub status: Option<u16>,
}

impl std::fmt::Display for GithubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn require_token() -> Result<String, GithubError> {
    match secrets::get_secret(KEY_GITHUB_TOKEN) {
        Ok(Some(t)) if !t.is_empty() => Ok(t),
        Ok(_) => Err(GithubError {
            message: "请先在设置中配置 GitHub Token".into(),
            status: Some(401),
        }),
        Err(e) => Err(GithubError {
            message: format!("Keychain: {e}"),
            status: Some(500),
        }),
    }
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// `path` is relative to `https://api.github.com/` (no leading slash).
pub fn request(method: Method, path: &str, body: Option<&Value>) -> Result<Value, GithubError> {
    request_inner(method, path, body, false)
}

fn request_inner(
    method: Method,
    path: &str,
    body: Option<&Value>,
    retried: bool,
) -> Result<Value, GithubError> {
    let token = require_token()?;
    let url = if path.starts_with("http") {
        path.to_string()
    } else {
        format!("{API_BASE}/{path}")
    };
    let mut req = client()
        .request(method.clone(), &url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "lulu-workbench");
    if let Some(b) = body {
        req = req.json(b);
    }
    let resp = req.send().map_err(|e| GithubError {
        message: format!("HTTP: {e}"),
        status: None,
    })?;
    let status = resp.status();
    if status.as_u16() == 429 && !retried {
        std::thread::sleep(Duration::from_millis(500));
        return request_inner(method, path, body, true);
    }
    let text = resp.text().map_err(|e| GithubError {
        message: format!("read body: {e}"),
        status: Some(status.as_u16()),
    })?;
    if status.is_success() {
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| GithubError {
            message: format!("JSON: {e}"),
            status: Some(status.as_u16()),
        })
    } else {
        Err(GithubError {
            message: if text.trim().is_empty() {
                format!("GitHub API {status}")
            } else {
                text
            },
            status: Some(status.as_u16()),
        })
    }
}

pub fn get_contents(owner: &str, repo: &str, path: &str, ref_name: Option<&str>) -> Result<Value, GithubError> {
    let path = path.trim_start_matches('/');
    let mut api = format!("repos/{owner}/{repo}/contents/{path}");
    if let Some(r) = ref_name {
        api.push_str(&format!("?ref={}", urlencoding::encode(r)));
    }
    request(Method::GET, &api, None)
}

pub fn put_contents(
    owner: &str,
    repo: &str,
    path: &str,
    message: &str,
    content_utf8: &str,
    sha: Option<&str>,
) -> Result<Value, GithubError> {
    let path = path.trim_start_matches('/');
    let b64 = base64::engine::general_purpose::STANDARD.encode(content_utf8.as_bytes());
    let mut body = serde_json::json!({
        "message": message,
        "content": b64,
    });
    if let Some(s) = sha {
        body["sha"] = serde_json::json!(s);
    }
    let api = format!("repos/{owner}/{repo}/contents/{path}");
    request(Method::PUT, &api, Some(&body))
}

pub fn delete_contents(
    owner: &str,
    repo: &str,
    path: &str,
    message: &str,
    sha: &str,
) -> Result<Value, GithubError> {
    let path = path.trim_start_matches('/');
    let body = serde_json::json!({
        "message": message,
        "sha": sha,
    });
    let api = format!("repos/{owner}/{repo}/contents/{path}");
    request(Method::DELETE, &api, Some(&body))
}

pub fn get_tree_recursive(owner: &str, repo: &str, tree_ref: &str) -> Result<Value, GithubError> {
    let api = format!(
        "repos/{owner}/{repo}/git/trees/{}?recursive=1",
        urlencoding::encode(tree_ref)
    );
    request(Method::GET, &api, None)
}

pub fn list_user_repos_page(page: u32) -> Result<Value, GithubError> {
    request(
        Method::GET,
        &format!("user/repos?per_page=100&affiliation=owner&page={page}"),
        None,
    )
}

/// Decode base64 `content` field from Contents API response.
pub fn decode_contents_payload(item: &Value) -> Result<String, GithubError> {
    let content = item
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GithubError {
            message: "missing content".into(),
            status: None,
        })?;
    let cleaned = content.replace('\n', "");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|e| GithubError {
            message: format!("base64: {e}"),
            status: None,
        })?;
    String::from_utf8(bytes).map_err(|e| GithubError {
        message: format!("utf8: {e}"),
        status: None,
    })
}

pub fn error_json(err: &GithubError) -> Value {
    let status = err.status.unwrap_or(500);
    serde_json::json!({
        "error": err.message,
        "_status": status
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_token_fails_when_empty() {
        crate::config::secrets::test_secrets_clear();
        let err = require_token().expect_err("token");
        assert!(err.message.contains("GitHub Token"));
    }

    #[test]
    fn decode_contents_payload_roundtrip() {
        let raw = "hello";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
        let v = serde_json::json!({ "content": format!("{b64}\n") });
        let text = decode_contents_payload(&v).expect("decode");
        assert_eq!(text, "hello");
    }
}
