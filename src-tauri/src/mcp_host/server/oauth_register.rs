//! RFC 7591 dynamic client registration. Public clients only, no secret.

use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{json, Value};

use super::oauth_grant::json_status;
use super::oauth_log::agent;
use crate::services::auth::{shared, AuthError, RegisteredClient};
use crate::services::mcp_oauth_log::{clip, info, trace_for, warn};

const MAX_BODY: usize = 16 * 1024;

pub(super) async fn register(request: Request) -> Response {
    let user_agent = agent(request.headers());
    let outcome = match to_bytes(request.into_body(), MAX_BODY).await {
        Ok(bytes) => register_client(&bytes),
        Err(_) => Err(("invalid_client_metadata", Vec::new())),
    };
    match outcome {
        Ok(client) => {
            info(
                "client.register",
                &trace_for(&client.client_id),
                json!({
                    "outcome": "created",
                    "client_id": client.client_id,
                    "client_name": clip(&client.client_name),
                    "redirect_uris": uris_for_log(&client.redirect_uris),
                    "user_agent": user_agent,
                }),
            );
            created(json!({
                "client_id": client.client_id,
                "client_name": client.client_name,
                "redirect_uris": client.redirect_uris,
                "grant_types": ["authorization_code"],
                "response_types": ["code"],
                "token_endpoint_auth_method": "none",
            }))
        }
        Err((error, sent_uris)) => {
            warn(
                "client.register",
                &trace_for(""),
                json!({
                    "outcome": "rejected",
                    "error": error,
                    "redirect_uris": uris_for_log(&sent_uris),
                    "user_agent": user_agent,
                }),
            );
            json_status(StatusCode::BAD_REQUEST, error)
        }
    }
}

/// On refusal: the OAuth error code, plus the redirect URIs the client sent (for the log).
fn register_client(bytes: &[u8]) -> Result<RegisteredClient, (&'static str, Vec<String>)> {
    let Ok(body) = serde_json::from_slice::<Value>(bytes) else {
        return Err(("invalid_client_metadata", Vec::new()));
    };
    let Some(redirect_uris) = string_list(body.get("redirect_uris")) else {
        return Err(("invalid_redirect_uri", Vec::new()));
    };
    let name = body
        .get("client_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    shared()
        .register_client(name, &redirect_uris)
        .map_err(|error| {
            let code = match error {
                AuthError::invalid_redirect_uri => "invalid_redirect_uri",
                _ => "invalid_client_metadata",
            };
            (code, redirect_uris.clone())
        })
}

/// Clipped, and capped at the registration limit so a hostile body cannot flood the log.
fn uris_for_log(uris: &[String]) -> Vec<String> {
    uris.iter().take(8).map(|uri| clip(uri)).collect()
}

/// `None` when the field is missing, not an array, or holds a non-string.
fn string_list(value: Option<&Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn created(body: Value) -> Response {
    (
        StatusCode::CREATED,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body.to_string(),
    )
        .into_response()
}
