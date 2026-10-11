//! Per-request `_meta` carrier for the diagnostic trace id on `tools/call`.
//!
//! The id travels in the request body instead of an HTTP header so it stays
//! correct when one connection serves many calls.

use rmcp::model::RequestMetaObject;
use serde_json::Value;

use crate::agent::diagnostics::TraceId;

/// `_meta` key that holds the trace id.
pub(crate) const TRACE_META_KEY: &str = "spark/trace_id";

/// Build the `_meta` object a `tools/call` carries so the server logs the same
/// trace id as the client.
pub(crate) fn build_trace_meta(trace_id: &TraceId) -> RequestMetaObject {
    let mut meta = RequestMetaObject::new();
    meta.insert(
        TRACE_META_KEY.to_string(),
        Value::String(trace_id.as_str().to_string()),
    );
    meta
}

/// Read the trace id back from a request's `_meta`. `None` when absent or not a
/// valid trace id, so the caller can mint its own.
pub(crate) fn read_trace_id(meta: &RequestMetaObject) -> Option<TraceId> {
    meta.get(TRACE_META_KEY)
        .and_then(Value::as_str)
        .and_then(TraceId::parse)
}

#[cfg(test)]
#[path = "../../unit-tests/agent/mcp_trace_meta_tests.rs"]
mod tests;
