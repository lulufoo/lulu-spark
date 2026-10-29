//! MCP Host consent and token routes. Same origin as the MCP server.

use std::collections::HashMap;

use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::oauth_log::agent;
use crate::services::auth::{shared, token_body, AuthError, CodeIssue, TokenRequest};
use crate::services::mcp_oauth::Slot;
use crate::services::mcp_oauth_log::{clip, info, trace_for, warn};
use crate::services::oauth_consent::oauth_consent_html_for;

const INVALID_PAGE: &str = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Lulu Spark</title></head><body><h1>Invalid authorization request</h1>\
<p>Start the connection again from your MCP client.</p></body></html>";

pub(super) async fn consent_page(request: Request) -> Response {
    let query = parse_form(request.uri().query().unwrap_or(""));
    let client_id = query.get("client_id").map(String::as_str).unwrap_or("");
    let redirect_uri = query.get("redirect_uri").map(String::as_str).unwrap_or("");
    let resource = query.get("resource").map(String::as_str).unwrap_or("");
    let trace = trace_for(client_id);
    let method = query.get("code_challenge_method").map_or("", String::as_str);
    let resource_slot = slot_from_resource(resource);
    let mut fields = json!({
        "client_id": clip(client_id),
        "redirect_uri": clip(redirect_uri),
        "resource": clip(resource),
        "slot": resource_slot.unwrap_or(Slot::Codex).as_str(),
        "resource_defaulted": resource_slot.is_none(),
        "code_challenge_method": clip(method),
        "has_code_challenge": query.get("code_challenge").is_some_and(|c| !c.is_empty()),
        "user_agent": agent(request.headers()),
    });
    match shared().authorize_client(client_id, redirect_uri) {
        Ok(name) => {
            fields["outcome"] = json!("shown");
            fields["client_name"] = json!(clip(&name));
            info("consent.page", &trace, fields);
            html(StatusCode::OK, oauth_consent_html_for(&name))
        }
        Err(_) => {
            fields["outcome"] = json!("rejected");
            warn("consent.page", &trace, fields);
            html(StatusCode::BAD_REQUEST, INVALID_PAGE.to_string())
        }
    }
}

pub(super) async fn consent_finish(request: Request) -> Response {
    let user_agent = agent(request.headers());
    let form = match form_body(request).await {
        Ok(form) => form,
        Err(response) => {
            let fields = json!({ "outcome": "bad_body", "user_agent": user_agent });
            warn("consent.decision", &trace_for(""), fields);
            return response;
        }
    };
    let redirect_uri = form.get("redirect_uri").map(String::as_str).unwrap_or("");
    let client_id = form.get("client_id").map(String::as_str).unwrap_or("");
    let trace = trace_for(client_id);
    let decision = |outcome: &str, extra: serde_json::Value| {
        let mut fields = json!({
            "outcome": outcome,
            "client_id": clip(client_id),
            "redirect_uri": clip(redirect_uri),
            "user_agent": user_agent,
        });
        if let (Some(base), Some(more)) = (fields.as_object_mut(), extra.as_object()) {
            base.extend(more.clone());
        }
        fields
    };
    if shared().authorize_client(client_id, redirect_uri).is_err() {
        warn("consent.decision", &trace, decision("rejected", json!({})));
        return json_status(StatusCode::BAD_REQUEST, "invalid_request");
    }
    let state = form.get("state").map(String::as_str);
    if form.get("decision").map(String::as_str) != Some("allow") {
        info("consent.decision", &trace, decision("denied", json!({})));
        return redirect(&error_location(redirect_uri, "access_denied", state));
    }
    let challenge = form.get("code_challenge").cloned().unwrap_or_default();
    let resource = form.get("resource").map(String::as_str).unwrap_or("");
    let resource_slot = slot_from_resource(resource);
    let slot = resource_slot.unwrap_or(Slot::Codex);
    let slot_fields = json!({
        "slot": slot.as_str(),
        "resource": clip(resource),
        "resource_defaulted": resource_slot.is_none(),
    });
    match shared().issue_code(CodeIssue::s256(client_id, redirect_uri, challenge, slot)) {
        Ok(code) => {
            info("consent.decision", &trace, decision("code_issued", slot_fields));
            redirect(&code_location(redirect_uri, &code, state))
        }
        Err(error) => {
            let code = if error == AuthError::invalid_request {
                "invalid_request"
            } else {
                "invalid_grant"
            };
            let failed = decision("issue_failed", json!({ "error": code }));
            warn("consent.decision", &trace, failed);
            json_status(StatusCode::BAD_REQUEST, code)
        }
    }
}

pub(super) async fn token(request: Request) -> Response {
    let user_agent = agent(request.headers());
    let form = match form_body(request).await {
        Ok(form) => form,
        Err(response) => {
            let fields = json!({ "outcome": "bad_body", "user_agent": user_agent });
            warn("token.request", &trace_for(""), fields);
            return response;
        }
    };
    let field = |key: &str| form.get(key).map(String::as_str).unwrap_or("");
    info(
        "token.request",
        &trace_for(field("client_id")),
        json!({
            "grant_type": clip(field("grant_type")),
            "client_id": clip(field("client_id")),
            "redirect_uri": clip(field("redirect_uri")),
            "has_code": !field("code").is_empty(),
            "has_code_verifier": !field("code_verifier").is_empty(),
            "user_agent": user_agent,
        }),
    );
    let body = TokenRequest {
        grant_type: form.get("grant_type").cloned().unwrap_or_default(),
        code: form.get("code").cloned().unwrap_or_default(),
        code_verifier: form.get("code_verifier").cloned().unwrap_or_default(),
        redirect_uri: form.get("redirect_uri").cloned().unwrap_or_default(),
        client_id: form.get("client_id").cloned().unwrap_or_default(),
    };
    match shared().exchange_token(body) {
        Ok(grant) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            token_body(&grant).to_string(),
        )
            .into_response(),
        Err(AuthError::invalid_request) => json_status(StatusCode::BAD_REQUEST, "invalid_request"),
        Err(AuthError::unsupported_grant) => {
            json_status(StatusCode::BAD_REQUEST, "unsupported_grant_type")
        }
        Err(_) => json_status(StatusCode::BAD_REQUEST, "invalid_grant"),
    }
}

async fn form_body(request: Request) -> Result<HashMap<String, String>, Response> {
    let bytes = to_bytes(request.into_body(), 64 * 1024)
        .await
        .map_err(|_| json_status(StatusCode::BAD_REQUEST, "invalid_request"))?;
    let raw = String::from_utf8(bytes.to_vec())
        .map_err(|_| json_status(StatusCode::BAD_REQUEST, "invalid_request"))?;
    Ok(parse_form(&raw))
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

/// `None` when `resource` is missing or names no known slot; callers fall back to Codex.
fn slot_from_resource(resource: &str) -> Option<Slot> {
    let path = resource.split(['?', '#']).next().unwrap_or("");
    let name = path.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    Slot::parse(name).ok()
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

fn redirect(location: &str) -> Response {
    (
        StatusCode::FOUND,
        [(header::LOCATION, location.to_string())],
    )
        .into_response()
}

fn html(status: StatusCode, body: String) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

pub(super) fn json_status(status: StatusCode, error: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({ "error": error }).to_string(),
    )
        .into_response()
}
