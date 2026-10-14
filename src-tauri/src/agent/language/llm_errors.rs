//! Copy for failed LLM calls.

use super::Language;
use crate::agent::llm::LlmError;

/// What the user is told when an LLM call fails. One text per error; the model's own
/// fields (partial text, `finish_reason`) are kept elsewhere.
pub fn describe_llm_error(language: Language, error: &LlmError) -> String {
    match language {
        Language::English => describe_in_english(error),
    }
}

fn describe_in_english(error: &LlmError) -> String {
    match error {
        LlmError::MissingConfig => {
            "Host / GLM is not configured. Enter the API key and model in Settings.".into()
        }
        LlmError::Unauthorized => "Authentication failed. Check the API key in Settings.".into(),
        LlmError::BadRequest(_) => {
            "The upstream service rejected the request. Try again later or check the configuration."
                .into()
        }
        LlmError::RateLimited => "Too many requests. Try again later.".into(),
        LlmError::Server(_) | LlmError::Network(_) => {
            "The upstream service is temporarily unavailable. Try again later.".into()
        }
        LlmError::Timeout => "The call timed out. Try again later.".into(),
        LlmError::TotalTimeout => {
            "The response took too long and was stopped. Shorten the request or try again later."
                .into()
        }
        LlmError::Truncated => "The reply was cut off. Try again or shorten the request.".into(),
        LlmError::ThinkingExhausted => {
            "The model spent its whole output budget on thinking and gave no reply. Shorten the request or try again later."
                .into()
        }
        LlmError::ContentFiltered => {
            "The reply was blocked by the upstream safety review. Adjust the request and try again."
                .into()
        }
        LlmError::InferenceFailed => "The model hit an inference error. Try again later.".into(),
        LlmError::ContextExceeded => {
            "The conversation is longer than the model's context window. Shorten the request or start a new chat."
                .into()
        }
        LlmError::UnknownFinish(reason) => format!(
            "The model stopped for an unknown reason ({reason}) and gave no reply. Nothing was written."
        ),
        LlmError::UnsupportedToolCalls => {
            "The upstream service does not support tool calls (tool_calls), so the turn cannot continue."
                .into()
        }
        LlmError::InvalidResponse(_) => {
            "The model's response was malformed. Nothing was written.".into()
        }
    }
}
