//! Decide whether a finished stream is an acceptable response (LW-171).
//!
//! The raw `finish_reason` string is never replaced by an enum; only this step maps it
//! to a `LlmError`. `stop` and `tool_calls` are the only values every engine agrees on.

use super::LlmError;
use crate::agent::session;

/// What arrived, reduced to the facts the decision needs.
pub(super) struct Shape<'a> {
    pub has_content: bool,
    pub has_tool_call: bool,
    pub has_reasoning: bool,
    /// A tool call without an id or a name.
    pub malformed_tool_calls: bool,
    pub finish_reason: Option<&'a str>,
}

/// Longest unrecognised `finish_reason` kept in an error or log line.
const RAW_LIMIT: usize = 64;

pub(super) fn classify_finish(shape: &Shape) -> Option<LlmError> {
    let no_output = !shape.has_content && !shape.has_tool_call;
    match shape.finish_reason {
        // Always an error: a cut-off answer or cut-off tool arguments is not usable.
        Some("length") => {
            let _ = session::log_agent_error("LLM finish_reason=length (truncated)");
            if no_output && shape.has_reasoning {
                return Some(LlmError::ThinkingExhausted);
            }
            return Some(LlmError::Truncated);
        }
        // GLM values with a known meaning: always an error, even with partial text.
        Some("sensitive") => return Some(logged(LlmError::ContentFiltered, "sensitive")),
        Some("network_error") => return Some(logged(LlmError::InferenceFailed, "network_error")),
        Some("model_context_window_exceeded") => {
            return Some(logged(LlmError::ContextExceeded, "model_context_window_exceeded"));
        }
        _ => {}
    }
    if shape.malformed_tool_calls {
        return Some(LlmError::InvalidResponse("malformed tool_calls".into()));
    }
    match shape.finish_reason {
        Some("stop") | Some("tool_calls") => None,
        None if no_output => Some(LlmError::InvalidResponse("missing choices".into())),
        None => None,
        // Unknown value (other engines, or new ones): only an error when nothing usable
        // arrived. With text or tool calls the response is used as it is.
        Some(other) => {
            let raw = clip(other);
            let _ = session::log_agent_error(&format!("LLM unrecognized finish_reason={raw}"));
            no_output.then(|| LlmError::UnknownFinish(raw))
        }
    }
}

fn logged(error: LlmError, finish_reason: &str) -> LlmError {
    let _ = session::log_agent_error(&format!("LLM finish_reason={finish_reason}"));
    error
}

fn clip(raw: &str) -> String {
    raw.chars().take(RAW_LIMIT).collect()
}
