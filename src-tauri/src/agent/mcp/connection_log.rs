//! Diagnostic events for the live MCP connection (`assistant.mcp`).
//!
//! They answer, from a session's own `session.log`, whether a turn handshook,
//! whether `tools/list` reused the connection, and why a connection was
//! dropped. Events without a chat session land in the process-level app log.
//!
//! Two scopes, matching the id layers in `agent::diagnostics`:
//! [`McpTurn`] is one user message and logs turn-level events; [`McpTrace`] is
//! one MCP request and logs request-level events with its own `trace_id`.

use std::time::Duration;

use crate::agent::diagnostics::{self, DiagnosticEvent, TraceId, TurnId};
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

/// One turn of one chat session: where turn-level connection events are
/// written, and the source of every request's [`McpTrace`].
#[derive(Debug, Clone)]
pub struct McpTurn {
    turn_id: TurnId,
    session_id: String,
}

impl McpTurn {
    pub fn new(turn_id: &TurnId, session_id: &str) -> Self {
        Self {
            turn_id: turn_id.clone(),
            session_id: session_id.to_string(),
        }
    }

    /// A new request of this turn, with a trace id of its own.
    pub fn trace(&self) -> McpTrace {
        McpTrace {
            turn_id: self.turn_id.clone(),
            session_id: self.session_id.clone(),
            trace_id: TraceId::new(),
        }
    }

    /// The live connection of another config was dropped before this turn's request.
    pub(super) fn connection_reset(&self, reason: ResetReason) {
        emit(
            &self.session_id,
            DiagnosticEvent::point(COMPONENT, "connection.reset", &self.turn_id)
                .with_static_field("reason", reason.as_str()),
        );
    }
}

/// One MCP request. It always knows its turn and its chat session, so a line
/// with a `trace_id` can never lack a `turn_id` or a `session_id`; the only way
/// to get one is [`McpTurn::trace`].
#[derive(Debug, Clone)]
pub struct McpTrace {
    turn_id: TurnId,
    session_id: String,
    trace_id: TraceId,
}

impl McpTrace {
    /// The turn this request belongs to.
    pub(super) fn turn(&self) -> McpTurn {
        McpTurn {
            turn_id: self.turn_id.clone(),
            session_id: self.session_id.clone(),
        }
    }

    /// `_meta` of this request: session, turn and trace id, so the server logs
    /// the same ids as the client.
    pub(super) fn meta(&self) -> RequestMetaObject {
        build_trace_meta(&self.trace_id, &self.turn_id, &self.session_id)
    }

    pub(super) fn connect_started(&self) {
        self.emit(DiagnosticEvent::point(COMPONENT, "connect.started", &self.turn_id));
    }

    pub(super) fn connect_completed(&self, elapsed: Duration, connected: bool) {
        self.emit(
            DiagnosticEvent::timing(COMPONENT, "connect.completed", &self.turn_id, elapsed)
                .with_static_field("outcome", outcome(connected)),
        );
    }

    /// This request failed and the live connection was dropped.
    pub(super) fn connection_reset(&self, reason: ResetReason) {
        self.emit(
            DiagnosticEvent::point(COMPONENT, "connection.reset", &self.turn_id)
                .with_static_field("reason", reason.as_str()),
        );
    }

    /// One `tools/list` request. `reused`: it went over an existing connection
    /// rather than a freshly handshaken one. `attempt` is 1, or 2 for the retry.
    pub(super) fn list_attempt_completed(
        &self,
        elapsed: Duration,
        attempt: u64,
        reused: bool,
        result: &Result<ToolCatalog, McpError>,
    ) {
        let mut event =
            DiagnosticEvent::timing(COMPONENT, "list.attempt.completed", &self.turn_id, elapsed)
                .with_u64_field("attempt", attempt)
                .with_bool_field("reused", reused)
                .with_static_field("outcome", outcome(result.is_ok()));
        event = match result {
            Ok(catalog) => event.with_u64_field("tool_count", catalog.definitions.len() as u64),
            Err(error) => event.with_bounded_text_field("sdk_error_message", &error.to_string()),
        };
        self.emit(event);
    }

    pub(crate) fn call_started(&self, tool_name: &str) {
        self.emit(
            DiagnosticEvent::point(COMPONENT, "call.request.started", &self.turn_id)
                .with_bounded_text_field("tool_name", tool_name),
        );
    }

    pub(crate) fn call_completed(&self, elapsed: Duration, tool_name: &str, ok: bool) {
        self.emit(
            DiagnosticEvent::timing(COMPONENT, "call.request.completed", &self.turn_id, elapsed)
                .with_bounded_text_field("tool_name", tool_name)
                .with_static_field("outcome", outcome(ok)),
        );
    }

    fn emit(&self, event: DiagnosticEvent) {
        emit(&self.session_id, event.with_trace_id(&self.trace_id));
    }
}

/// The live connection was released by the process (Binding Reset, app exit),
/// outside any turn: no ids, only the reason, in the app log.
pub(super) fn connection_released(reason: ResetReason) {
    let _ = diagnostics::log(
        DiagnosticEvent::process_point(COMPONENT, "connection.reset")
            .with_static_field("reason", reason.as_str()),
    );
}

fn emit(session_id: &str, event: DiagnosticEvent) {
    // An empty session id stays unset and falls through to the app log.
    let _ = diagnostics::log(event.with_session_id(session_id));
}

fn outcome(ok: bool) -> &'static str {
    if ok { "ok" } else { "error" }
}
