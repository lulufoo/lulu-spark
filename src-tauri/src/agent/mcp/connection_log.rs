//! Diagnostic events for the live MCP connection (`assistant.mcp`).
//!
//! They answer, from a session's own `session.log`, whether a turn handshook,
//! whether `tools/list` reused the connection, and why a connection was
//! dropped. Events without a chat session land in the process-level app log.

use std::time::Duration;

use crate::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::agent::tools::catalog::ToolCatalog;

use rmcp::model::RequestMetaObject;

use super::{build_trace_meta, McpError};

const COMPONENT: &str = "assistant.mcp";

/// Why the live connection was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetReason {
    ListFailed,
    CallFailed,
    ConfigChanged,
    BindingReset,
    AppExit,
}

impl ResetReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::ListFailed => "list_failed",
            Self::CallFailed => "call_failed",
            Self::ConfigChanged => "config_changed",
            Self::BindingReset => "binding_reset",
            Self::AppExit => "app_exit",
        }
    }
}

/// Where connection events are written: one turn of one chat session.
#[derive(Debug, Clone)]
pub struct McpTrace {
    trace_id: TraceId,
    session_id: String,
}

impl McpTrace {
    pub fn new(trace_id: &TraceId, session_id: &str) -> Self {
        Self {
            trace_id: trace_id.clone(),
            session_id: session_id.to_string(),
        }
    }

    /// Not tied to any chat session (Binding Reset, app exit): events go to the
    /// process-level app log.
    pub fn detached() -> Self {
        Self::new(&TraceId::new(), "")
    }

    /// `_meta` for every request of this turn: both ids, so the server can log them.
    pub(super) fn meta(&self) -> RequestMetaObject {
        build_trace_meta(&self.trace_id, &self.session_id)
    }

    pub(super) fn connect_started(&self) {
        self.emit(DiagnosticEvent::point(COMPONENT, "connect.started", &self.trace_id));
    }

    pub(super) fn connect_completed(&self, elapsed: Duration, connected: bool) {
        self.emit(
            DiagnosticEvent::timing(COMPONENT, "connect.completed", &self.trace_id, elapsed)
                .with_static_field("outcome", outcome(connected)),
        );
    }

    /// One `tools/list` attempt sequence. `reused`: the first request went over
    /// an existing connection. `retried`: it failed and was sent once more.
    pub(super) fn list_completed(
        &self,
        elapsed: Duration,
        reused: bool,
        retried: bool,
        result: &Result<ToolCatalog, McpError>,
    ) {
        let mut event =
            DiagnosticEvent::timing(COMPONENT, "list.completed", &self.trace_id, elapsed)
                .with_bool_field("reused", reused)
                .with_bool_field("retried", retried)
                .with_static_field("outcome", outcome(result.is_ok()));
        if let Ok(catalog) = result {
            event = event.with_u64_field("tool_count", catalog.definitions.len() as u64);
        }
        self.emit(event);
    }

    pub(super) fn connection_reset(&self, reason: ResetReason) {
        self.emit(
            DiagnosticEvent::point(COMPONENT, "connection.reset", &self.trace_id)
                .with_static_field("reason", reason.as_str()),
        );
    }

    fn emit(&self, event: DiagnosticEvent) {
        // An empty session id stays unset and falls through to the app log.
        let _ = diagnostics::log(event.with_session_id(&self.session_id));
    }
}

fn outcome(ok: bool) -> &'static str {
    if ok { "ok" } else { "error" }
}
