//! Resolve display title from URL (`/api/fetch-title`).

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{json, Value};

pub fn fetch_link_title(url: &str) -> Value {
    let url = url.trim();
    if url.is_empty() {
        return json!({ "error": "missing url", "_status": 400 });
    }
    match resolve_title(url) {
        Ok(title) => json!({ "title": title }),
        Err(e) => json!({ "error": e, "_status": 500 }),
    }
}

fn resolve_title(url: &str) -> Result<String, String> {
    Ok(fallback_title(url))
}

fn is_safe_https_host(url: &str) -> bool {
    if !url.starts_with("https://") {
        return false;
    }
    let after = url.strip_prefix("https://").unwrap_or("");
    let host = after.split('/').next().unwrap_or("");
    let host = host.split('@').next_back().unwrap_or(host);
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return false;
    }
    if host.starts_with("192.168.") || host.starts_with("10.") {
        return false;
    }
    if host.starts_with("172.") {
        if let Some(second) = host.split('.').nth(1).and_then(|s| s.parse::<u8>().ok()) {
            if (16..=31).contains(&second) {
                return false;
            }
        }
    }
    true
}

fn fallback_title(url: &str) -> String {
    let path = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('?')
        .next()
        .unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or(url);
    let decoded = urlencoding::decode(name).unwrap_or_else(|_| name.into());
    decoded.trim_end_matches(".md").to_string()
}

/// Generic HTTPS fetch + parse `<title>` (10s timeout).
pub fn fetch_html_title(url: &str) -> Result<String, String> {
    if !is_safe_https_host(url) {
        return Err("blocked host".into());
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let html = resp.text().map_err(|e| e.to_string())?;
    parse_title_tag(&html).ok_or_else(|| "no title".into())
}

fn parse_title_tag(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    Some(html[start..end].trim().to_string())
}

#[cfg(test)]
#[path = "../unit-tests/services/link_title.rs"]
mod tests;
