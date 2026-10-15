//! Copy for replies the engine writes itself during a Turn.

use super::catalog::read_text;
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

/// The content service (the scene's MCP server) has no tools or can't be reached. Its own
/// error text is not shown; it stays in the logs.
pub fn describe_content_unavailable(language: Language) -> &'static str {
    read_text(language, "turn.content_unavailable")
}

/// The model asked for a tool call this scene cannot run. `wrote` is true when a tool
/// already changed something earlier in the Turn.
pub fn describe_unusable_response(language: Language, wrote: bool) -> &'static str {
    let key = if wrote {
        "turn.unusable_response_after_write"
    } else {
        "turn.unusable_response"
    };
    read_text(language, key)
}

pub fn describe_clarify_limit(language: Language) -> &'static str {
    read_text(language, "turn.clarify_limit")
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
