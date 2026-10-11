//! Per-request `_meta` carrier for the diagnostic trace id and chat session id.
//!
//! `initialize`, `tools/list` and `tools/call` each carry them in the request
//! body instead of an HTTP header, so they stay correct when one connection
//! serves many turns.

use rmcp::model::RequestMetaObject;
use serde_json::Value;

use crate::agent::diagnostics::{sanitize_session_id, TraceId};

/// `_meta` key that holds the trace id.
pub(crate) const TRACE_META_KEY: &str = "spark/trace_id";
/// `_meta` key that holds the chat session id.
pub(crate) const SESSION_META_KEY: &str = "spark/session_id";

/// What a request's `_meta` says about where it came from. Each part is `None`
/// when absent or invalid, so the reader can fall back (e.g. mint its own trace id).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct TraceMeta {
    pub(crate) trace_id: Option<TraceId>,
    pub(crate) session_id: Option<String>,
}

/// Build the `_meta` a request carries so the server logs the same ids as the
/// client. An empty or invalid `session_id` is left out.
pub(crate) fn build_trace_meta(trace_id: &TraceId, session_id: &str) -> RequestMetaObject {
    let mut meta = RequestMetaObject::new();
    meta.insert(
        TRACE_META_KEY.to_string(),
        Value::String(trace_id.as_str().to_string()),
    );
    if let Some(session_id) = sanitize_session_id(session_id) {
        meta.insert(SESSION_META_KEY.to_string(), Value::String(session_id));
    }
    meta
}

/// Read the ids back from rmcp's request `_meta`.
pub(crate) fn read_trace_meta(meta: &RequestMetaObject) -> TraceMeta {
    read_values(meta.get(TRACE_META_KEY), meta.get(SESSION_META_KEY))
}

fn read_values(trace_id: Option<&Value>, session_id: Option<&Value>) -> TraceMeta {
    TraceMeta {
        trace_id: trace_id.and_then(Value::as_str).and_then(TraceId::parse),
        session_id: session_id.and_then(Value::as_str).and_then(sanitize_session_id),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/mcp_trace_meta_tests.rs"]
mod tests;
