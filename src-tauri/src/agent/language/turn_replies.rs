//! Copy for replies the engine writes itself during a Turn.

use super::catalog::{fill_text, read_text};
use super::Language;

/// A response that ended with no text and no tool calls. `wrote` is true once a mutating
/// tool succeeded in this Turn; `tool_rounds` counts the tool rounds already run. A model
/// that stops silently after writing must not be reported as "nothing was written".
pub fn describe_empty_reply(language: Language, wrote: bool, tool_rounds: usize) -> &'static str {
    let key = if wrote {
        "turn.empty_reply_wrote"
    } else if tool_rounds > 0 {
        "turn.empty_reply_tools_only"
    } else {
        "turn.empty_reply_nothing"
    };
    read_text(language, key)
}

/// The last LLM call of a Turn produced no text.
pub fn describe_no_wrap_up(language: Language) -> &'static str {
    read_text(language, "turn.no_wrap_up")
}

pub fn describe_mcp_without_tools(language: Language) -> &'static str {
    read_text(language, "turn.mcp_without_tools")
}

/// `error` is the service's own text and is passed on as it is.
pub fn describe_mcp_unavailable(language: Language, error: &str) -> String {
    fill_text(language, "turn.mcp_unavailable", &[("error", error)])
}

/// The model answered with something the turn cannot use, or asked for a tool that does
/// not exist here.
pub fn describe_unusable_response(language: Language) -> &'static str {
    read_text(language, "turn.unusable_response")
}

pub fn describe_clarify_limit(language: Language) -> &'static str {
    read_text(language, "turn.clarify_limit")
}

/// No usable model settings: the user has to configure the service first.
pub fn describe_not_configured(language: Language) -> &'static str {
    read_text(language, "turn.not_configured")
}

/// A chat was started while no business scene is active.
pub fn describe_unbound(language: Language) -> &'static str {
    read_text(language, "turn.unbound")
}

/// The scene has no prompt to run with.
pub fn describe_empty_prompt(language: Language) -> &'static str {
    read_text(language, "turn.empty_prompt")
}

/// The turn was cut off by a cancel or a scene change.
pub fn describe_cancelled(language: Language) -> &'static str {
    read_text(language, "turn.cancelled")
}

/// Another turn is still running.
pub fn describe_busy(language: Language) -> &'static str {
    read_text(language, "turn.busy")
}

/// The turn's session is no longer the live one.
pub fn describe_session_mismatch(language: Language) -> &'static str {
    read_text(language, "turn.session_mismatch")
}
