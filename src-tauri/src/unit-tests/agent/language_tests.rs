use super::{
    describe_busy, describe_cancelled, describe_clarify_limit, describe_content_unavailable,
    describe_empty_prompt, describe_empty_reply, describe_llm_error, describe_no_wrap_up,
    describe_preparing_action, describe_replying, describe_replying_preview, describe_retrying,
    describe_running_action, describe_session_mismatch, describe_thinking, describe_unbound,
    describe_unusable_response, describe_working, Language,
};
use crate::agent::llm::LlmError;

/// Words that name the engine's insides. None may reach the user.
const INTERNAL_TERMS: [&str; 9] = [
    "host", "glm", "mcp", "llm", "tool_calls", "finish_reason", "inference", "upstream", "binding",
];

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

const LANGUAGES: [Language; 2] = [Language::English, Language::ChineseSimplified];

/// Every message the user can see in `language`, whatever the inputs.
fn every_message(language: Language) -> Vec<String> {
    let mut texts = Vec::new();
    for wrote in [false, true] {
        for error in every_llm_error() {
            texts.push(describe_llm_error(language, &error, wrote));
        }
        texts.push(describe_unusable_response(language, wrote).to_string());
    }
    for (wrote, rounds) in [(true, 1), (false, 1), (false, 0)] {
        texts.push(describe_empty_reply(language, wrote, rounds).to_string());
    }
    texts.extend(
        [
            describe_no_wrap_up(language),
            describe_content_unavailable(language),
            describe_clarify_limit(language),
            describe_unbound(language),
            describe_empty_prompt(language),
            describe_cancelled(language),
            describe_busy(language),
            describe_session_mismatch(language),
            describe_working(language),
            describe_preparing_action(language),
            describe_running_action(language),
            describe_replying(language),
            describe_thinking(language),
            describe_retrying(language),
        ]
        .map(str::to_string),
    );
    texts.push(describe_replying_preview(language, "preview"));
    texts
}

#[test]
fn every_message_has_copy_and_no_leftover_variable_in_every_language() {
    for language in LANGUAGES {
        for text in every_message(language) {
            assert!(!text.trim().is_empty(), "{}", language.code());
            assert!(!text.contains('{') && !text.contains('}'), "{text}");
        }
    }
}

#[test]
fn no_message_uses_an_internal_term_in_any_language() {
    for language in LANGUAGES {
        for text in every_message(language) {
            let lower = text.to_lowercase();
            for term in INTERNAL_TERMS {
                assert!(!lower.contains(term), "`{term}` in {}: {text}", language.code());
            }
        }
    }
}

#[test]
fn the_chinese_copy_is_chinese_and_differs_from_the_english() {
    let english = every_message(Language::English);
    let chinese = every_message(Language::ChineseSimplified);
    assert_eq!(english.len(), chinese.len());
    for (en, zh) in english.iter().zip(&chinese) {
        assert!(zh.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{zh}");
        assert_ne!(en, zh);
    }
}

#[test]
fn error_replies_say_only_plain_ascii() {
    for wrote in [false, true] {
        for error in every_llm_error() {
            let text = describe_llm_error(Language::English, &error, wrote);
            assert!(text.is_ascii(), "{error:?}: {text}");
        }
    }
}

#[test]
fn the_details_of_an_error_are_not_in_its_copy() {
    for language in LANGUAGES {
        let unknown = LlmError::UnknownFinish("odd".into());
        assert!(!describe_llm_error(language, &unknown, false).contains("odd"));
        let network = LlmError::Network("refused".into());
        assert!(!describe_llm_error(language, &network, false).contains("refused"));
    }
}

#[test]
fn a_promise_of_no_changes_is_only_made_when_nothing_was_written() {
    for (language, phrase) in [
        (Language::English, "No changes were made"),
        (Language::ChineseSimplified, "没有做任何修改"),
    ] {
        let promising = |text: &str| text.contains(phrase);
        for error in [
            LlmError::UnknownFinish("odd".into()),
            LlmError::InvalidResponse("x".into()),
        ] {
            assert!(promising(&describe_llm_error(language, &error, false)), "{error:?}");
            assert!(!promising(&describe_llm_error(language, &error, true)), "{error:?}");
        }
        assert!(promising(describe_unusable_response(language, false)));
        assert!(!promising(describe_unusable_response(language, true)));
        assert!(!promising(describe_empty_reply(language, true, 3)));
    }
}

#[test]
fn the_reply_preview_is_part_of_the_hint() {
    assert_eq!(describe_replying_preview(Language::English, "你好"), "Replying… 你好");
    assert_eq!(describe_replying_preview(Language::ChineseSimplified, "你好"), "正在回复… 你好");
}
