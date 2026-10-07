//! Notify-chain hops on the process execution log (`app_log`).
//!
//! Not the Assistant diagnostic file: that allowlist is chat-only.

use serde_json::{json, Map, Value};

use crate::services::app_log::{self, Level, Side};

pub const TRACE_QUERY: &str = "trace";
pub const TRACE_PARAM: &str = "trace_id";
pub const BUSINESS: &str = "os-notify";
pub const NODE_MESSAGE_CENTER_RECEIVED: &str = "message_center.received";
pub const NODE_NOTIFY_SEND: &str = "notify.send";
pub const NODE_CLICK_NATIVE: &str = "click.native";
pub const NODE_SCHEME_OPEN: &str = "scheme.open";

pub use crate::services::app_log::{new_trace_id, parse_trace_id};

const QUERY_KEYS: &[&str] = &["id", "path", "trace", "date", "note", "layer"];
const SCHEME_MAX: usize = 512;

pub fn trace_from_scheme(scheme: &str) -> Option<&str> {
    scheme.split(['?', '&']).find_map(|part| {
        let raw = part.strip_prefix("trace=")?;
        parse_trace_id(raw)
    })
}

pub fn scheme_query_fields(scheme: &str) -> Map<String, Value> {
    let mut map = Map::new();
    let clipped = if scheme.len() > SCHEME_MAX {
        &scheme[..SCHEME_MAX]
    } else {
        scheme
    };
    map.insert("scheme".into(), json!(clipped));
    if let Some((_, query)) = scheme.split_once('?') {
        for part in query.split('&') {
            let Some((key, raw)) = part.split_once('=') else {
                continue;
            };
            if !QUERY_KEYS.contains(&key) {
                continue;
            }
            let decoded = urlencoding::decode(raw)
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|_| raw.to_string());
            map.insert(key.to_string(), json!(decoded));
        }
    }
    map
}

pub fn log_hop(node: &str, trace_id: &str, outcome: &str) {
    log_hop_extra(node, trace_id, outcome, None, Side::Host);
}

pub fn log_hop_extra(
    node: &str,
    trace_id: &str,
    outcome: &str,
    extra: Option<Value>,
    side: Side,
) {
    let id = parse_trace_id(trace_id).unwrap_or("trace_missing");
    let mut params = match extra {
        Some(Value::Object(fields)) => fields,
        _ => Map::new(),
    };
    params.insert("outcome".into(), json!(outcome));
    app_log::log(
        BUSINESS,
        node,
        Some(id),
        Some(Value::Object(params)),
        side,
        Level::Info,
    );
}

pub fn log_scheme_open(scheme: &str) {
    if crate::services::login_hop::try_log_scheme_open(scheme) {
        return;
    }
    let trace = trace_from_scheme(scheme).unwrap_or("trace_missing");
    log_hop_extra(
        NODE_SCHEME_OPEN,
        trace,
        if scheme.is_empty() { "empty" } else { "ok" },
        Some(Value::Object(scheme_query_fields(scheme))),
        Side::Native,
    );
}

pub fn log_click_native(scheme: &str, has_app: bool) {
    let outcome = if scheme.is_empty() {
        "empty"
    } else if !has_app {
        "no_app"
    } else {
        "ok"
    };
    log_hop_extra(
        NODE_CLICK_NATIVE,
        trace_from_scheme(scheme).unwrap_or("trace_missing"),
        outcome,
        Some(Value::Object(scheme_query_fields(scheme))),
        Side::Native,
    );
}

#[cfg(test)]
#[path = "../unit-tests/services/os_notify_trace.rs"]
mod tests;
