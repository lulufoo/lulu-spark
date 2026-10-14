use std::cell::Cell;

use super::run_with_retries;
use crate::agent::llm::{AssistantMessage, LlmCallOutcome, LlmError};

fn failed(error: LlmError) -> LlmCallOutcome {
    LlmCallOutcome {
        snapshot: None,
        error: Some(error),
    }
}

fn answered() -> LlmCallOutcome {
    LlmCallOutcome {
        snapshot: Some(AssistantMessage::default()),
        error: None,
    }
}

/// Sends the scripted outcomes in order and counts how many were asked for.
fn run(
    script: Vec<LlmCallOutcome>,
    max_retries: usize,
    is_cancelled: Option<&dyn Fn() -> bool>,
) -> (LlmCallOutcome, usize) {
    let sent = Cell::new(0);
    let mut script = script.into_iter();
    let outcome = run_with_retries(max_retries, is_cancelled, || {
        sent.set(sent.get() + 1);
        script.next().expect("sent more requests than scripted")
    });
    (outcome, sent.get())
}

#[test]
fn a_timeout_then_an_answer_is_one_success_after_two_requests() {
    let (outcome, sent) = run(vec![failed(LlmError::Timeout), answered()], 2, None);
    assert_eq!(outcome.error, None);
    assert_eq!(sent, 2);
}

#[test]
fn the_original_and_two_retries_all_timing_out_ends_in_timeout() {
    let script = vec![
        failed(LlmError::Timeout),
        failed(LlmError::Timeout),
        failed(LlmError::Timeout),
    ];
    let (outcome, sent) = run(script, 2, None);
    assert_eq!(outcome.error, Some(LlmError::Timeout));
    assert_eq!(sent, 3);
}

#[test]
fn no_retry_budget_means_one_request() {
    let (outcome, sent) = run(vec![failed(LlmError::Timeout)], 0, None);
    assert_eq!(outcome.error, Some(LlmError::Timeout));
    assert_eq!(sent, 1);
}

#[test]
fn only_a_plain_timeout_is_sent_again() {
    for error in [
        LlmError::TotalTimeout,
        LlmError::Server(503),
        LlmError::RateLimited,
        LlmError::Network("reset".into()),
        LlmError::Unauthorized,
        LlmError::BadRequest("no".into()),
    ] {
        let (outcome, sent) = run(vec![failed(error.clone())], 2, None);
        assert_eq!(outcome.error, Some(error.clone()));
        assert_eq!(sent, 1, "{error:?} must not be retried");
    }
}

#[test]
fn cancelling_between_attempts_stops_the_retries() {
    let cancelled = || true;
    let (outcome, sent) = run(vec![failed(LlmError::Timeout)], 2, Some(&cancelled));
    assert_eq!(outcome.error, Some(LlmError::Timeout));
    assert_eq!(sent, 1);
}
