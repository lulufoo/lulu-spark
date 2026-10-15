//! Copy for failed LLM calls.

use super::catalog::read_text;
use super::Language;
use crate::agent::llm::LlmError;

/// What the user is told when an LLM call fails. One text per error. `wrote` is true when
/// a tool already changed something earlier in the Turn: the errors that promise "no
/// changes were made" then say so differently. The service's own error text and the
/// `finish_reason` are not shown; they stay in the logs and call records.
pub fn describe_llm_error(language: Language, error: &LlmError, wrote: bool) -> String {
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
        LlmError::UnknownFinish(_) if wrote => "llm_error.unknown_finish_after_write",
        LlmError::UnknownFinish(_) => "llm_error.unknown_finish",
        LlmError::UnsupportedToolCalls => "llm_error.unsupported_tool_calls",
        LlmError::InvalidResponse(_) if wrote => "llm_error.invalid_response_after_write",
        LlmError::InvalidResponse(_) => "llm_error.invalid_response",
    };
    read_text(language, key).to_string()
}
