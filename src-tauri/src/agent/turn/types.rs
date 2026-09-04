//! Shared Loop types, terminals, and Host event names.

use serde::Serialize;
use serde_json::Value;

pub const EVENT_TURN_COMPLETED: &str = "ai-assistant:turn-completed";
pub const WINDOW_LABEL: &str = "ai-assistant";
pub const MAX_CLARIFY_ROUNDS: u32 = 5;
pub const MAX_HISTORY_MESSAGES: usize = 2000;
pub const MAX_USER_TURNS: usize = 200;
pub const MAX_MCP_TOOL_ROUNDS: usize = 25;
pub const MAX_MCP_TOOL_CALLS: usize = 25;
/// Concurrent in-flight chat turns (one row per session_id).
pub const MAX_CHAT_FLIGHTS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    None,
    Business,
    Error,
}

impl Terminal {
    pub fn as_str(self) -> &'static str {
        match self {
            Terminal::None => "none",
            Terminal::Business => "business",
            Terminal::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub reply_text: String,
    pub terminal: Terminal,
    pub wrote: bool,
}

#[derive(Debug, Clone)]
pub struct ChatTurnResult {
    pub body: Value,
    pub emit_turn_completed: Option<Value>,
}

/// Binding Contract lifecycle callback payload (E1): event name + optional result category only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleEvent {
    pub event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
}

/// Shell sync notice equivalent to `ai-assistant:binding-changed` (Bound/Unbound).
/// Recorded on Host cut/Set paths so core-only calls cannot bypass shell sync observability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellSyncEvent {
    pub event: &'static str,
    pub state: &'static str,
}

/// Same event name the shell listens for (`commands::ai_assistant::EVENT_BINDING_CHANGED`).
pub const EVENT_SHELL_BINDING_CHANGED: &str = "ai-assistant:binding-changed";

/// Outcome of a gated Binding Contract execute (tools/prompt from current Binding only).
#[derive(Debug, Clone)]
pub struct ExecuteOutcome {
    pub applied_tools: Value,
    pub applied_prompt: Value,
}

/// Stable execute / gate failure categories (L11-AR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecError {
    code: &'static str,
}

impl ExecError {
    pub fn as_code(&self) -> &'static str {
        self.code
    }

    pub(crate) fn rejected_unbound() -> Self {
        Self {
            code: "rejected_unbound",
        }
    }

    pub(crate) fn reset_cancelled() -> Self {
        Self {
            code: "reset_cancelled",
        }
    }

    pub(crate) fn rejected_stale_generation() -> Self {
        Self {
            code: "rejected_stale_generation",
        }
    }
}

/// Outcome of Host Present (shell open). Not a Binding Contract result.
/// `window_label` is a Host surface id — it does not imply a WebviewWindow exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentOutcome {
    pub surface: &'static str,
    pub window_label: &'static str,
    pub entry_id: &'static str,
}
