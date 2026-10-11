//! Read-only peek at an MCP request for logging.
//!
//! The HTTP gate logs before rmcp parses the body, so it reads the JSON-RPC
//! `method` and the `_meta` trace/session ids itself, then hands the body on
//! untouched. This is only used to label log lines; it never changes routing
//! or authentication.

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::{header, Method, StatusCode};
use serde_json::{json, Value};

use crate::agent::mcp::{read_trace_meta_json, TraceMeta};

/// Bodies larger than this (or of unknown length) are forwarded unread.
const PEEK_LIMIT: usize = 1 << 20;

/// What the logs know about one request.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct RequestInfo {
    /// JSON-RPC method, or the HTTP verb when the body was not peeked.
    pub(super) method: String,
    pub(super) meta: TraceMeta,
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
    Some(RequestInfo { method, meta })
}

/// Fields of an HTTP-level log line: `fields` plus the component, the method
/// and, when the client sent one, the chat session id.
pub(super) fn http_log_params(info: &RequestInfo, fields: Value) -> Option<Value> {
    let mut params = json!({ "component": "mcp_host.http", "method": info.method });
    if let (Some(target), Some(extra)) = (params.as_object_mut(), fields.as_object()) {
        target.extend(extra.clone());
    }
    Some(with_session_id(params, info.meta.session_id.as_deref()))
}

/// `value` plus `session_id` when there is one.
pub(super) fn with_session_id(mut value: Value, session_id: Option<&str>) -> Value {
    if let (Some(session_id), Some(object)) = (session_id, value.as_object_mut()) {
        object.insert("session_id".into(), Value::String(session_id.to_string()));
    }
    value
}

#[cfg(test)]
#[path = "../../unit-tests/mcp_host/request_meta_tests.rs"]
mod tests;
