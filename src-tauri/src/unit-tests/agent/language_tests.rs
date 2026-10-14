use super::{
    describe_clarify_limit, describe_empty_reply, describe_llm_error, describe_mcp_unavailable,
    describe_mcp_without_tools, describe_no_wrap_up, describe_unusable_response, Language,
};
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

#[test]
fn every_turn_reply_has_english_copy_without_other_scripts() {
    let language = Language::English;
    let texts = [
        describe_empty_reply(language, true, 1).to_string(),
        describe_empty_reply(language, false, 1).to_string(),
        describe_empty_reply(language, false, 0).to_string(),
        describe_no_wrap_up(language).to_string(),
        describe_mcp_without_tools(language).to_string(),
        describe_mcp_unavailable(language, "connection refused"),
        describe_unusable_response(language).to_string(),
        describe_clarify_limit(language).to_string(),
    ];
    for text in texts {
        assert!(!text.trim().is_empty());
        assert!(text.is_ascii(), "{text}");
    }
}

#[test]
fn the_mcp_error_text_is_passed_on_as_it_is() {
    let text = describe_mcp_unavailable(Language::English, "connection refused");
    assert!(text.ends_with(": connection refused"), "{text}");
}
