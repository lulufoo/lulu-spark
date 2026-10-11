//! Correlation ids of the Assistant diagnostics.
//!
//! Three layers, each contained in the one above:
//!
//! * `session_id` — one chat session.
//! * [`TurnId`] — one user message inside a session. Every diagnostic line of the turn
//!   carries it.
//! * [`TraceId`] — one request that crosses a process boundary (an MCP request). Unique
//!   per request, so a client line and the server line of the same request share it.
//!
//! A line that carries a `trace_id` also carries its `turn_id` and `session_id`; a line
//! that carries a `turn_id` also carries its `session_id`. The reverse does not hold.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::services::id::random_hex12;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// 12 hex chars. `random_hex12` hashes the clock and the thread, so two calls in the same
/// clock tick on one thread repeat; the low 16 bits here are a process-wide counter, so
/// ids minted back to back never do.
fn unique_hex12() -> String {
    let count = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let head = u64::from_str_radix(&random_hex12(), 16).unwrap_or_default();
    format!("{:012x}", (head & 0xFFFF_FFFF_0000) | (count & 0xFFFF))
}

/// Accept only bounded identifier characters so externally supplied values cannot inject
/// line breaks or arbitrary text into the log.
fn parse_id(value: &str) -> Option<String> {
    let value = value.trim();
    if !(8..=64).contains(&value.len())
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(value.to_string())
}

/// One user message inside a chat session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnId(String);

impl TurnId {
    pub fn parse(value: &str) -> Option<Self> {
        parse_id(value).map(Self)
    }

    pub fn new() -> Self {
        Self(format!("turn_{}", unique_hex12()))
    }

    pub fn from_optional(value: Option<String>) -> Self {
        value
            .as_deref()
            .and_then(Self::parse)
            .unwrap_or_else(Self::new)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One request that crosses a process boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceId(String);

impl TraceId {
    pub fn parse(value: &str) -> Option<Self> {
        parse_id(value).map(Self)
    }

    pub fn new() -> Self {
        Self(format!("trace_{}", unique_hex12()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
