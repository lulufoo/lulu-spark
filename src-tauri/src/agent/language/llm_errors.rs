//! Copy for failed LLM calls.

use super::catalog::{fill_text, read_text};
use super::Language;
use crate::agent::llm::LlmError;

/// What the user is told when an LLM call fails. One text per error; the model's own
/// fields (partial text, `finish_reason`) are kept elsewhere.
pub fn describe_llm_error(language: Language, error: &LlmError) -> String {
    let key = match error {
        LlmError::MissingConfig => "llm_error.missing_config",
        LlmError::Unauthorized => "llm_error.unauthorized",
        LlmError::BadRequest(_) => "llm_error.bad_request",
        LlmError::RateLimited => "llm_error.rate_limited",
        LlmError::Server(_) | LlmError::Network(_) => "llm_error.unavailable",
        LlmError::Timeout => "llm_error.timeout",
        LlmError::TotalTimeout => "llm_error.total_timeout",
        LlmError::Truncated => "llm_error.truncated",
        LlmError::ThinkingExhausted => "llm_error.thinking_exhausted",
        LlmError::ContentFiltered => "llm_error.content_filtered",
        LlmError::InferenceFailed => "llm_error.inference_failed",
        LlmError::ContextExceeded => "llm_error.context_exceeded",
        LlmError::UnknownFinish(reason) => {
            return fill_text(language, "llm_error.unknown_finish", &[("reason", reason)]);
        }
        LlmError::UnsupportedToolCalls => "llm_error.unsupported_tool_calls",
        LlmError::InvalidResponse(_) => "llm_error.invalid_response",
    };
    read_text(language, key).to_string()
}
