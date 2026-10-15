use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::{run_with_timeout, Waited, LOCAL_TOOL_TIMEOUT};

fn never_cancelled() -> bool {
    false
}

#[test]
fn the_local_tool_limit_is_ten_seconds() {
    assert_eq!(LOCAL_TOOL_TIMEOUT, Duration::from_secs(10));
}

#[test]
fn work_that_finishes_in_time_returns_its_value() {
    let waited = run_with_timeout(Duration::from_secs(5), &never_cancelled, |_| 7);
    assert_eq!(waited, Waited::Done(7));
}

#[test]
fn work_that_is_too_slow_times_out_without_waiting_for_it() {
    let started = Instant::now();
    let waited = run_with_timeout(Duration::from_millis(50), &never_cancelled, |_| {
        thread::sleep(Duration::from_millis(600));
        7
    });
    assert_eq!(waited, Waited::TimedOut);
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "the caller must not wait for the slow work"
    );
}

#[test]
fn cancel_ends_the_wait_before_the_limit() {
    let started = Instant::now();
    let waited = run_with_timeout(Duration::from_secs(5), &|| true, |_| {
        thread::sleep(Duration::from_millis(600));
        7
    });
    assert_eq!(waited, Waited::Cancelled);
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "cancel must be seen within a poll or two"
    );
}

#[test]
fn work_that_panics_is_reported_as_crashed() {
    let waited: Waited<i32> =
        run_with_timeout(Duration::from_secs(5), &never_cancelled, |_| panic!("boom"));
    assert_eq!(waited, Waited::Crashed);
}

#[test]
fn work_receives_a_deadline_one_limit_from_now() {
    let before = Instant::now();
    let waited = run_with_timeout(Duration::from_secs(2), &never_cancelled, |deadline| deadline);
    let after = Instant::now();
    let Waited::Done(deadline) = waited else {
        panic!("work should finish in time");
    };
    assert!(deadline >= before + Duration::from_secs(2));
    assert!(deadline <= after + Duration::from_secs(2));
}

#[test]
fn abandoned_work_keeps_running_to_the_end() {
    // A thread cannot be stopped from outside: after a timeout the work still finishes.
    let finished = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&finished);
    let waited = run_with_timeout(Duration::from_millis(30), &never_cancelled, move |_| {
        thread::sleep(Duration::from_millis(200));
        flag.store(true, Ordering::SeqCst);
    });
    assert_eq!(waited, Waited::TimedOut);
    assert!(!finished.load(Ordering::SeqCst));
    thread::sleep(Duration::from_millis(500));
    assert!(finished.load(Ordering::SeqCst));
}
