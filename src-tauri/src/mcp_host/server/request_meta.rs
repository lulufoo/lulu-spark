//! Read-only peek at an MCP request for logging.
//!
//! The HTTP gate logs before rmcp parses the body, so it reads the JSON-RPC
//! `method` and the `_meta` turn/session ids itself, then hands the body on
//! untouched. This is only used to label log lines; it never changes routing
//! or authentication.

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::{header, Method, StatusCode};
use serde_json::{json, Value};

use crate::agent::mcp::{read_trace_meta_json, TraceMeta};

use super::caller::{clean_label, Caller};

/// Bodies larger than this (or of unknown length) are forwarded unread.
const PEEK_LIMIT: usize = 1 << 20;
const CLIENT_NAME_MAX: usize = 64;
const CLIENT_VERSION_MAX: usize = 32;

/// What the logs know about one request.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct RequestInfo {
    /// JSON-RPC method, or the HTTP verb when the body was not peeked.
    pub(super) method: String,
    pub(super) meta: TraceMeta,
    /// `clientInfo` name and version, read from `initialize` only.
    pub(super) client_name: Option<String>,
    pub(super) client_version: Option<String>,
}

/// Peek at `request` and return it, with its body intact, plus what was seen.
/// Fails only when a body that declared a small length cannot be read.
pub(super) async fn peek_request(request: Request) -> Result<(Request, RequestInfo), StatusCode> {
    let verb = request.method().to_string();
    if request.method() != Method::POST || !declares_small_body(&request) {
        return Ok((request, RequestInfo { method: verb, ..Default::default() }));
    }
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, PEEK_LIMIT)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let info = describe_body(&bytes).unwrap_or(RequestInfo {
        method: verb,
        ..Default::default()
    });
    Ok((Request::from_parts(parts, Body::from(bytes)), info))
}

fn declares_small_body(request: &Request) -> bool {
    request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length <= PEEK_LIMIT)
}

/// `None` when the body is not a JSON-RPC request object (not JSON, a batch
/// array, a response).
fn describe_body(bytes: &[u8]) -> Option<RequestInfo> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let method = value.get("method")?.as_str()?.to_string();
    let meta = value
        .pointer("/params/_meta")
        .map(read_trace_meta_json)
        .unwrap_or_default();
    let client = |field: &str, max: usize| {
        (method == "initialize")
            .then(|| value.pointer(&format!("/params/clientInfo/{field}")))
            .flatten()
            .and_then(Value::as_str)
            .and_then(|text| clean_label(text, max))
    };
    Some(RequestInfo {
        client_name: client("name", CLIENT_NAME_MAX),
        client_version: client("version", CLIENT_VERSION_MAX),
        method,
        meta,
    })
}

/// Fields of an HTTP-level log line: `fields` plus the component, the method,
/// who is calling (slot, user agent, Bearer fingerprint, and on `initialize`
/// the client's name and version) and, when the client sent them, the chat
/// session id and turn id.
pub(super) fn http_log_params(info: &RequestInfo, caller: &Caller, fields: Value) -> Option<Value> {
    let mut params = json!({ "component": "mcp_host.http", "method": info.method });
    if let Some(target) = params.as_object_mut() {
        caller.add_to(target);
        for (name, value) in [("client_name", &info.client_name), ("client_version", &info.client_version)] {
            if let Some(value) = value {
                target.insert(name.into(), Value::String(value.clone()));
            }
        }
        if let Some(extra) = fields.as_object() {
            target.extend(extra.clone());
        }
    }
    Some(with_meta_ids(params, &info.meta))
}

/// `value` plus the `session_id` and `turn_id` the client sent, when it sent them.
pub(super) fn with_meta_ids(mut value: Value, meta: &TraceMeta) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };
    if let Some(session_id) = &meta.session_id {
        object.insert("session_id".into(), Value::String(session_id.clone()));
    }
    if let Some(turn_id) = &meta.turn_id {
        object.insert("turn_id".into(), Value::String(turn_id.as_str().to_string()));
    }
    value
}

#[cfg(test)]
#[path = "../../unit-tests/mcp_host/request_meta_tests.rs"]
mod tests;
