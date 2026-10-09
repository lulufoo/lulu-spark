//! LW-171: which finished streams are errors, and which error they are.

use super::classify::{classify_finish, Shape};
use super::LlmError;
use crate::test_support::TestSandbox;

fn shape(finish: Option<&str>) -> Shape<'_> {
    Shape {
        has_content: false,
        has_tool_call: false,
        has_reasoning: false,
        malformed_tool_calls: false,
        finish_reason: finish,
    }
}

fn classify(shape: Shape) -> Option<LlmError> {
    // Classifying writes to agent-error.log, so keep it inside a sandbox.
    let _sandbox = TestSandbox::new();
    classify_finish(&shape)
}

#[test]
fn length_with_only_thinking_means_the_budget_was_used_up() {
    let only_thinking = Shape { has_reasoning: true, ..shape(Some("length")) };
    assert_eq!(classify(only_thinking), Some(LlmError::ThinkingExhausted));
}

#[test]
fn length_is_always_an_error_and_stays_truncated_otherwise() {
    let with_text = Shape { has_content: true, has_reasoning: true, ..shape(Some("length")) };
    assert_eq!(classify(with_text), Some(LlmError::Truncated));
    let with_call = Shape { has_tool_call: true, ..shape(Some("length")) };
    assert_eq!(classify(with_call), Some(LlmError::Truncated));
    assert_eq!(classify(shape(Some("length"))), Some(LlmError::Truncated));
    // Length wins over a malformed call: the call was cut off.
    let cut_off = Shape { has_tool_call: true, malformed_tool_calls: true, ..shape(Some("length")) };
    assert_eq!(classify(cut_off), Some(LlmError::Truncated));
}

#[test]
fn known_glm_failures_are_errors_even_with_partial_text() {
    let partial = |finish| Shape { has_content: true, ..shape(Some(finish)) };
    assert_eq!(classify(partial("sensitive")), Some(LlmError::ContentFiltered));
    assert_eq!(classify(partial("network_error")), Some(LlmError::InferenceFailed));
    assert_eq!(
        classify(partial("model_context_window_exceeded")),
        Some(LlmError::ContextExceeded)
    );
}

#[test]
fn unknown_finish_reason_with_output_is_used_as_it_is() {
    let with_text = Shape { has_content: true, ..shape(Some("end_turn")) };
    assert_eq!(classify(with_text), None);
    let with_call = Shape { has_tool_call: true, ..shape(Some("function_call")) };
    assert_eq!(classify(with_call), None);
}

#[test]
fn unknown_finish_reason_without_output_is_an_error_carrying_the_raw_value() {
    assert_eq!(
        classify(shape(Some("end_turn"))),
        Some(LlmError::UnknownFinish("end_turn".into()))
    );
    let thinking = Shape { has_reasoning: true, ..shape(Some("eos")) };
    assert_eq!(classify(thinking), Some(LlmError::UnknownFinish("eos".into())));
    let long = "x".repeat(200);
    let Some(LlmError::UnknownFinish(raw)) = classify(shape(Some(&long))) else {
        panic!("expected UnknownFinish");
    };
    assert_eq!(raw.chars().count(), 64);
}

#[test]
fn stop_with_nothing_is_not_an_error_here_the_loop_words_the_empty_reply() {
    assert_eq!(classify(shape(Some("stop"))), None);
    let thinking = Shape { has_reasoning: true, ..shape(Some("stop")) };
    assert_eq!(classify(thinking), None);
    assert_eq!(classify(shape(Some("tool_calls"))), None);
}

#[test]
fn missing_finish_reason_is_only_an_error_when_nothing_arrived() {
    assert_eq!(
        classify(shape(None)),
        Some(LlmError::InvalidResponse("missing choices".into()))
    );
    let with_text = Shape { has_content: true, ..shape(None) };
    assert_eq!(classify(with_text), None);
}

#[test]
fn malformed_tool_calls_are_invalid_for_normal_finish_reasons() {
    let malformed = Shape { has_tool_call: true, malformed_tool_calls: true, ..shape(Some("tool_calls")) };
    assert_eq!(
        classify(malformed),
        Some(LlmError::InvalidResponse("malformed tool_calls".into()))
    );
}
