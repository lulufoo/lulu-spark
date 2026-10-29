use std::collections::HashMap;

use serde_json::json;
use tiny_http::Method;

use crate::services::auth::{shared, AuthError, CodeIssue};
use crate::services::mcp_oauth::Slot;
use crate::services::oauth_consent::oauth_consent_html;

use super::respond::{respond_html, respond_json, respond_redirect};

pub(super) fn handle_oauth_consent(request: tiny_http::Request) {
    if request.method() == &Method::Get {
        respond_html(request, 200, oauth_consent_html());
        return;
    }
    if request.method() != &Method::Post {
        respond_json(request, 405, json!({ "error": "Method not allowed" }));
        return;
    }
    finish_consent(request);
}

fn finish_consent(mut request: tiny_http::Request) {
    let mut raw = String::new();
    if request.as_reader().read_to_string(&mut raw).is_err() {
        respond_json(request, 400, json!({ "error": "invalid_request" }));
        return;
    }
    let form = parse_form(&raw);
    let redirect_uri = form.get("redirect_uri").map(String::as_str).unwrap_or("");
    if !is_loopback_redirect(redirect_uri) {
        respond_json(request, 400, json!({ "error": "invalid_request" }));
        return;
    }
    let state = form.get("state").map(String::as_str);
    if form.get("decision").map(String::as_str) != Some("allow") {
        respond_redirect(request, &error_location(redirect_uri, "access_denied", state));
        return;
    }
    let client_id = form.get("client_id").cloned().unwrap_or_default();
    let challenge = form.get("code_challenge").cloned().unwrap_or_default();
    match shared().issue_code(CodeIssue::s256(client_id, redirect_uri, challenge, Slot::Codex)) {
        Ok(code) => respond_redirect(request, &code_location(redirect_uri, &code, state)),
        Err(AuthError::invalid_request) => {
            respond_json(request, 400, json!({ "error": "invalid_request" }))
        }
        Err(_) => respond_json(request, 400, json!({ "error": "invalid_grant" })),
    }
}

fn parse_form(raw: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in raw.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        if key.is_empty() {
            continue;
        }
        map.insert(decode(key), decode(value));
    }
    map
}

fn decode(value: &str) -> String {
    urlencoding::decode(value)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

fn is_loopback_redirect(uri: &str) -> bool {
    let rest = uri
        .strip_prefix("http://")
        .or_else(|| uri.strip_prefix("https://"));
    let Some(rest) = rest else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    host == "127.0.0.1" || host == "localhost"
}

fn code_location(redirect_uri: &str, code: &str, state: Option<&str>) -> String {
    let mut location = with_query(redirect_uri, "code", code);
    if let Some(state) = state.filter(|s| !s.is_empty()) {
        location = with_query(&location, "state", state);
    }
    location
}

fn error_location(redirect_uri: &str, error: &str, state: Option<&str>) -> String {
    let mut location = with_query(redirect_uri, "error", error);
    if let Some(state) = state.filter(|s| !s.is_empty()) {
        location = with_query(&location, "state", state);
    }
    location
}

fn with_query(uri: &str, key: &str, value: &str) -> String {
    let sep = if uri.contains('?') { '&' } else { '?' };
    format!("{uri}{sep}{key}={}", urlencoding::encode(value))
}
