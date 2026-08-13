//! Versioned, privacy-safe diagnostic events for Assistant execution timing.
//!
//! Events are emitted as one JSON object per line to stderr and to
//! `{cache_dir}/agent/assistant-diagnostic.jsonl`. Field names are static.
//! Prompt/reply bodies are still forbidden. Bounded, newline-stripped error
//! classification text is allowed so SDK failures can be distinguished.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};

use crate::services::agent::session;
use crate::services::id::random_hex12;

pub const SCHEMA_VERSION: u8 = 1;
pub const DIAGNOSTIC_LOG_FILE: &str = "assistant-diagnostic.jsonl";
pub const BOUNDED_TEXT_MAX: usize = 1024;

const BOUNDED_TEXT_FIELDS: &[&str] = &[
    "business_id",
    "sdk_error_name",
    "sdk_error_code",
    "sdk_error_message",
    "sdk_error_keys",
    "sdk_error_json",
    "sdk_cause_name",
    "sdk_cause_code",
    "sdk_cause_message",
    "sdk_wait_keys",
    "sdk_wait_json",
    "sdk_model",
    "stderr_preview",
];

static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Correlation identifier shared by the UI, Host command, and engine events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceId(String);

impl TraceId {
    /// Accept only bounded identifier characters so externally supplied values
    /// cannot inject line breaks or arbitrary text into the log.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if !(8..=64).contains(&value.len())
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return None;
        }
        Some(Self(value.to_string()))
    }

    pub fn new() -> Self {
        Self(format!("trace_{}", random_hex12()))
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

/// A single structured diagnostic event.
///
/// `fields` can only be populated through safe metadata helpers. Arbitrary
/// dynamic strings remain unsupported; bounded error-classification text is
/// allowed only for the SDK diagnostic field names below.
#[derive(Debug, Serialize)]
pub struct DiagnosticEvent {
    pub schema_version: u8,
    pub timestamp: String,
    pub component: &'static str,
    pub event: &'static str,
    pub trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    pub fields: BTreeMap<&'static str, Value>,
}

impl DiagnosticEvent {
    pub fn timing(
        component: &'static str,
        event: &'static str,
        trace_id: &TraceId,
        elapsed: Duration,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            timestamp: Utc::now().to_rfc3339(),
            component,
            event,
            trace_id: trace_id.as_str().to_string(),
            elapsed_ms: Some(elapsed.as_millis().try_into().unwrap_or(u64::MAX)),
            fields: BTreeMap::new(),
        }
    }

    pub fn point(component: &'static str, event: &'static str, trace_id: &TraceId) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            timestamp: Utc::now().to_rfc3339(),
            component,
            event,
            trace_id: trace_id.as_str().to_string(),
            elapsed_ms: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_static_field(mut self, name: &'static str, value: &'static str) -> Self {
        self.fields.insert(name, json!(value));
        self
    }

    pub fn with_bool_field(mut self, name: &'static str, value: bool) -> Self {
        self.fields.insert(name, json!(value));
        self
    }

    pub fn with_u64_field(mut self, name: &'static str, value: u64) -> Self {
        self.fields.insert(name, json!(value));
        self
    }

    /// Bounded error-classification text. Unknown field names are dropped so
    /// call sites cannot log prompts or replies through this helper.
    pub fn with_bounded_text_field(mut self, name: &'static str, value: &str) -> Self {
        if !BOUNDED_TEXT_FIELDS.contains(&name) {
            return self;
        }
        let sanitized = sanitize_bounded_text(value);
        if !sanitized.is_empty() {
            self.fields.insert(name, json!(sanitized));
        }
        self
    }
}

pub fn sanitize_bounded_text(value: &str) -> String {
    let collapsed: String = value
        .chars()
        .map(|c| {
            if c == '\n' || c == '\r' || c == '\t' || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let redacted = redact_sk_tokens(&session::redact_secrets(&collapsed)).replace(
        "CURSOR_API_KEY",
        "[REDACTED]",
    );
    let trimmed = redacted.trim();
    if trimmed.chars().count() <= BOUNDED_TEXT_MAX {
        trimmed.to_string()
    } else {
        trimmed.chars().take(BOUNDED_TEXT_MAX).collect()
    }
}

fn redact_sk_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find("sk-") {
        out.push_str(&rest[..idx]);
        out.push_str("sk-[REDACTED]");
        rest = &rest[idx + 3..];
        let skip = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(rest.len());
        rest = &rest[skip..];
    }
    out.push_str(rest);
    out
}

pub fn diagnostic_log_path() -> Result<PathBuf, String> {
    Ok(session::agent_log_dir()?.join(DIAGNOSTIC_LOG_FILE))
}

/// Emit a JSONL event to both stderr and the durable local diagnostics log.
pub fn log(event: DiagnosticEvent) -> Result<(), String> {
    let line = serde_json::to_string(&event).map_err(|e| e.to_string())?;
    eprintln!("[assistant-diagnostic] {line}");

    let path = diagnostic_log_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "diagnostic log path has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let lock = WRITE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())
}
