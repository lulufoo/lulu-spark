//! Copy for replies the engine writes itself during a Turn.

use super::Language;

/// A response that ended with no text and no tool calls. `wrote` is true once a mutating
/// tool succeeded in this Turn; `tool_rounds` counts the tool rounds already run. A model
/// that stops silently after writing must not be reported as "nothing was written".
pub fn describe_empty_reply(language: Language, wrote: bool, tool_rounds: usize) -> &'static str {
    match language {
        Language::English => {
            if wrote {
                "Writes were made, but the model gave no final reply."
            } else if tool_rounds > 0 {
                "Tools ran and nothing was written, but the model gave no final reply."
            } else {
                "The model's response was empty. Nothing was written."
            }
        }
    }
}

/// The last LLM call of a Turn produced no text.
pub fn describe_no_wrap_up(language: Language) -> &'static str {
    match language {
        Language::English => {
            "The LLM call limit for this turn was reached; nothing further was run."
        }
    }
}

pub fn describe_mcp_without_tools(language: Language) -> &'static str {
    match language {
        Language::English => {
            "The MCP service for this scene exposes no tools, so the turn cannot continue."
        }
    }
}

/// `error` is the service's own text and is passed on as it is.
pub fn describe_mcp_unavailable(language: Language, error: &str) -> String {
    match language {
        Language::English => format!("The MCP service for this scene is unavailable: {error}"),
    }
}

/// The model answered with something the turn cannot use, or asked for a tool that does
/// not exist here.
pub fn describe_unusable_response(language: Language) -> &'static str {
    match language {
        Language::English => {
            "The model's response was malformed or asked for an unsupported tool. Nothing was written."
        }
    }
}

pub fn describe_clarify_limit(language: Language) -> &'static str {
    match language {
        Language::English => {
            "Too many clarifying questions. Rephrase the request or try again later."
        }
    }
}
