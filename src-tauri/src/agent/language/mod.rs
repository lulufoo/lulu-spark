//! Fixed user-facing copy.
//!
//! The texts live in `src-tauri/locales/<code>.json`, one file per language (see
//! `catalog`). Convention: every message is one function; its first parameter is the
//! [`Language`]; its body picks the message key and fills the variables. The text itself
//! never appears in the code.

mod catalog;
mod llm_errors;
mod progress_hints;
mod turn_replies;

pub use llm_errors::describe_llm_error;
pub use progress_hints::{
    describe_preparing_action, describe_replying, describe_replying_preview,
    describe_retrying, describe_running_action, describe_thinking, describe_working,
};
pub use turn_replies::{
    describe_busy, describe_cancelled, describe_clarify_limit, describe_content_unavailable,
    describe_empty_prompt, describe_empty_reply, describe_no_wrap_up, describe_session_mismatch,
    describe_unbound, describe_unusable_response,
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
