//! Sending one logical LLM call again after a timeout.

use super::{LlmCallOutcome, LlmError};

/// Runs `send` once, then again while it ends in `LlmError::Timeout`, at most
/// `max_retries` more times and never once `is_cancelled` says so.
///
/// Only a connect or idle timeout is worth repeating. `TotalTimeout` would only repeat the
/// long wait; status and network errors are not sent again. Nothing the failed attempt
/// received is carried into the next one.
pub(super) fn run_with_retries(
    max_retries: usize,
    is_cancelled: Option<&dyn Fn() -> bool>,
    mut send: impl FnMut() -> LlmCallOutcome,
) -> LlmCallOutcome {
    let mut retries = 0;
    loop {
        let outcome = send();
        let timed_out = matches!(outcome.error, Some(LlmError::Timeout));
        let stopped = is_cancelled.is_some_and(|cancelled| cancelled());
        if !timed_out || retries >= max_retries || stopped {
            return outcome;
        }
        retries += 1;
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/llm_retry_pure_tests.rs"]
mod tests;
