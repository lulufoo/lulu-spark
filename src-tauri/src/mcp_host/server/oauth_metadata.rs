//! RFC 9728 protected-resource metadata for MCP clients.

use axum::extract::{Path, Request};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use serde_json::json;

use super::oauth_log::agent;
use super::types::REGISTERED_SCENE_SLOTS;
use crate::services::mcp_oauth_log::{info, trace_for, warn};

pub(super) const METADATA_PATH: &str = "/.well-known/oauth-protected-resource";
pub(super) const AUTHORIZATION_SERVER_PATH: &str = "/.well-known/oauth-authorization-server";

pub(super) fn www_authenticate(headers: &HeaderMap, scene_slot: &str) -> String {
    let host = request_host(headers);
    let meta = format!("http://{host}{METADATA_PATH}/mcp/{scene_slot}");
    format!("Bearer realm=\"Lulu Spark\", resource_metadata=\"{meta}\"")
}

pub(super) async fn authorization_server(request: Request) -> Response {
    let origin = origin(request.headers());
    let body = serde_json::json!({
        "issuer": origin,
        "authorization_endpoint": format!("{origin}/oauth/consent"),
        "token_endpoint": format!("{origin}/oauth/token"),
        "registration_endpoint": format!("{origin}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    });
    info(
        "metadata.server",
        &trace_for(""),
        json!({ "issuer": origin, "user_agent": agent(request.headers()) }),
    );
    json_ok(body)
}

pub(super) async fn protected_resource_root(request: Request) -> Response {
    log_resource_metadata(request.headers(), "root", "ok");
    metadata_response(request.headers(), None)
}

pub(super) async fn protected_resource_slot(
    Path(metadata_slot): Path<String>,
    request: Request,
) -> Response {
    if !is_metadata_slot(&metadata_slot) {
        log_resource_metadata(request.headers(), &metadata_slot, "not_found");
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::json!({ "error": "not_found" }).to_string(),
        )
            .into_response();
    }
    log_resource_metadata(request.headers(), &metadata_slot, "ok");
    metadata_response(request.headers(), Some(metadata_slot.as_str()))
}

/// RFC 9728 lookup: the step a client takes right after the 401 challenge.
fn log_resource_metadata(headers: &HeaderMap, path_slot: &str, outcome: &str) {
    let params = json!({
        "path_slot": crate::services::mcp_oauth_log::clip(path_slot),
        "outcome": outcome,
        "user_agent": agent(headers),
    });
    if outcome == "ok" {
        info("metadata.resource", &trace_for(""), params);
    } else {
        warn("metadata.resource", &trace_for(""), params);
    }
}

fn is_metadata_slot(scene_slot: &str) -> bool {
    scene_slot == "mobile" || REGISTERED_SCENE_SLOTS.contains(&scene_slot)
}

fn request_host(headers: &HeaderMap) -> String {
    headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or("127.0.0.1:9876")
        .to_string()
}

fn metadata_response(headers: &HeaderMap, scene_slot: Option<&str>) -> Response {
    let host = request_host(headers);
    let resource = match scene_slot {
        Some(slot) => format!("http://{host}/mcp/{slot}"),
        None => format!("http://{host}"),
    };
    let body = serde_json::json!({
        "resource": resource,
        "authorization_servers": [origin(headers)],
        "bearer_methods_supported": ["header"],
    });
    json_ok(body)
}

fn origin(headers: &HeaderMap) -> String {
    format!("http://{}", request_host(headers))
}

fn json_ok(body: serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
