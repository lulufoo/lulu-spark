use std::time::Duration;

use super::{connect_timeout_for, DEFAULT_TIMEOUT, LLM_CONNECT_TIMEOUT};

#[test]
fn connect_and_idle_timeouts_are_different_values() {
    assert_eq!(LLM_CONNECT_TIMEOUT, Duration::from_secs(30));
    assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(120));
    assert_eq!(connect_timeout_for(DEFAULT_TIMEOUT), LLM_CONNECT_TIMEOUT);
    assert_ne!(connect_timeout_for(DEFAULT_TIMEOUT), DEFAULT_TIMEOUT);
}

#[test]
fn connect_timeout_never_exceeds_a_shorter_caller_timeout() {
    let short = Duration::from_millis(200);
    assert_eq!(connect_timeout_for(short), short);
}
