//! Fixed user-facing copy.
//!
//! The texts live in `src-tauri/locales/<code>.json`, one file per language (see
//! `catalog`). Convention: every message is one function; its first parameter is the
//! [`Language`]; its body picks the message key and fills the variables. The text itself
//! never appears in the code.

mod catalog;
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
    /// The language code, which is also the name of its locale file.
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
        }
    }

    /// Only English exists today. A language setting would be read here, so callers
    /// that pass `Language::current()` need no change.
    pub fn current() -> Self {
        Language::English
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/language_tests.rs"]
mod tests;
