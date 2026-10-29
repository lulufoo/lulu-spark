//! Process-log lines for the MCP OAuth flow (`app_log`, business `mcp_oauth`).
//!
//! `app_log` tokens allow letters, digits, `.`, `_`, `-` only, so the business id
//! is `mcp_oauth`, not "MCP OAuth" (a space would drop every line).
//!
//! Log what a person needs to follow one connection attempt. Never log
//! authorization codes, PKCE challenge or verifier, `state`, or access tokens.

use serde_json::Value;

use crate::services::app_log::{self, new_trace_id, parse_trace_id, Level, Side};

pub const BUSINESS: &str = "mcp_oauth";
const FIELD_MAX: usize = 200;

/// One trace per client: its `client_id` joins register, consent and token lines.
/// A fresh trace is used before a client exists (discovery, malformed requests).
pub fn trace_for(client_id: &str) -> String {
    parse_trace_id(client_id)
        .map(str::to_string)
        .unwrap_or_else(new_trace_id)
}

/// Untrusted text made safe for a log field: control characters dropped, length capped.
pub fn clip(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(FIELD_MAX)
        .collect()
}

pub fn info(event: &str, trace_id: &str, params: Value) {
    emit(Level::Info, event, trace_id, params);
}

pub fn warn(event: &str, trace_id: &str, params: Value) {
    emit(Level::Warn, event, trace_id, params);
}

fn emit(level: Level, event: &str, trace_id: &str, params: Value) {
    app_log::log(BUSINESS, event, Some(trace_id), Some(params), Side::Host, level);
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_oauth_log.rs"]
mod tests;
