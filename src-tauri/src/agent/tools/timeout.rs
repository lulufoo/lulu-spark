//! Time limit for local (non-MCP) tool calls.
//!
//! The work runs on its own thread. The caller waits for it up to a limit and checks for
//! cancellation while it waits. A thread cannot be stopped from outside, so on timeout or
//! cancel it is abandoned and keeps running until the work returns by itself. Work that
//! loops (`grep`) is handed the deadline and stops on its own.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// How long one local tool call may run.
pub const LOCAL_TOOL_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the waiting caller looks at the cancel flag.
const CANCEL_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, PartialEq, Eq)]
pub enum Waited<T> {
    /// The work returned in time.
    Done(T),
    /// The limit passed first. The work may still be running.
    TimedOut,
    /// The caller was cancelled first. The work may still be running.
    Cancelled,
    /// The work panicked, or its thread could not be started.
    Crashed,
}

/// Run `work` on its own thread and wait for it for at most `limit`.
///
/// `work` receives the deadline so that loops inside it can stop by themselves.
pub fn run_with_timeout<T, F>(limit: Duration, is_cancelled: &dyn Fn() -> bool, work: F) -> Waited<T>
where
    T: Send + 'static,
    F: FnOnce(Instant) -> T + Send + 'static,
{
    let deadline = Instant::now() + limit;
    let (tx, rx) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("local-tool".into())
        .spawn(move || {
            let _ = tx.send(work(deadline));
        });
    if spawned.is_err() {
        return Waited::Crashed;
    }
    loop {
        let wait = CANCEL_POLL.min(deadline.saturating_duration_since(Instant::now()));
        match rx.recv_timeout(wait) {
            Ok(value) => return Waited::Done(value),
            Err(RecvTimeoutError::Timeout) => {
                if is_cancelled() {
                    return Waited::Cancelled;
                }
                if Instant::now() >= deadline {
                    return Waited::TimedOut;
                }
            }
            Err(RecvTimeoutError::Disconnected) => return Waited::Crashed,
        }
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_timeout_tests.rs"]
mod tests;
