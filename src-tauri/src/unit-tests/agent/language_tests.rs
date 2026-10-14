use super::{describe_llm_error, Language};
use crate::agent::llm::LlmError;

#[test]
fn the_current_language_is_english() {
    assert_eq!(Language::current(), Language::English);
}

fn every_llm_error() -> Vec<LlmError> {
    vec![
        LlmError::MissingConfig,
        LlmError::Unauthorized,
        LlmError::BadRequest("x".into()),
        LlmError::RateLimited,
        LlmError::Server(503),
        LlmError::Network("x".into()),
        LlmError::Timeout,
        LlmError::TotalTimeout,
        LlmError::Truncated,
        LlmError::ThinkingExhausted,
        LlmError::ContentFiltered,
        LlmError::InferenceFailed,
        LlmError::ContextExceeded,
        LlmError::UnknownFinish("odd".into()),
        LlmError::UnsupportedToolCalls,
        LlmError::InvalidResponse("x".into()),
    ]
}

#[test]
fn every_llm_error_has_english_copy_without_other_scripts() {
    for error in every_llm_error() {
        let text = describe_llm_error(Language::English, &error);
        assert!(!text.trim().is_empty(), "{error:?} has no copy");
        assert!(text.is_ascii(), "{error:?}: {text}");
    }
}

#[test]
fn an_unknown_finish_reason_is_named_in_the_copy() {
    let text = describe_llm_error(Language::English, &LlmError::UnknownFinish("odd".into()));
    assert!(text.contains("(odd)"), "{text}");
}
