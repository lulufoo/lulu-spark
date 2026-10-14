//! Fixed user-facing copy, one entry point per language.
//!
//! Convention: every message is one function; its first parameter is the [`Language`],
//! and its body is an exhaustive `match` on it. Adding a language then makes the
//! compiler list every message that still needs a text.

mod llm_errors;
mod turn_replies;

pub use llm_errors::describe_llm_error;
pub use turn_replies::{
    describe_clarify_limit, describe_empty_reply, describe_mcp_unavailable,
    describe_mcp_without_tools, describe_no_wrap_up, describe_unusable_response,
};

/// The language of the copy the engine writes itself (not of the model's replies).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
}

impl Language {
    /// Only English exists today. A language setting would be read here, so callers
    /// that pass `Language::current()` need no change.
    pub fn current() -> Self {
        Language::English
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/language_tests.rs"]
mod tests;
