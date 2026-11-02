//! One sync at a time. Two runs at once would send and write the same documents twice. The gate
//! also carries the user's wish to cancel the run that holds it, and the record of that run (see
//! [`SyncProgress`]), which it clears when a run begins and closes when the run ends.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use super::progress::SyncProgress;

const IDLE: u8 = 0;
const RUNNING: u8 = 1;
/// A run is going on and the user asked it to stop.
const CANCELLING: u8 = 2;

/// The app's one gate. Clones share it.
///
/// One state word, not a flag for "running" and another for "cancel": a cancel can then never be
/// lost between a run taking the gate and a flag being reset, and a cancel asked while nothing
/// runs can never hit the next run.
#[derive(Debug, Clone, Default)]
pub struct SyncGate {
    state: Arc<AtomicU8>,
    progress: SyncProgress,
}

/// Held for as long as a run goes on. Dropping it, also when the run panics, opens the gate.
#[derive(Debug)]
pub struct SyncRun {
    state: Arc<AtomicU8>,
    progress: SyncProgress,
}

impl SyncGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// `None` when a run is going on, and then the record of that run is left as it is.
    pub fn try_begin(&self) -> Option<SyncRun> {
        self.state.compare_exchange(IDLE, RUNNING, Ordering::AcqRel, Ordering::Acquire).ok().map(|_| {
            self.progress.begin();
            SyncRun { state: Arc::clone(&self.state), progress: self.progress.clone() }
        })
    }

    /// The record of the run that is going on, or of the last one.
    pub fn progress(&self) -> &SyncProgress {
        &self.progress
    }

    /// Asks the run that holds the gate to stop. Answers whether there is one: with no run going
    /// on nothing is remembered, so the next run is not cancelled by it. Asking twice is the same
    /// as asking once.
    pub fn cancel(&self) -> bool {
        self.state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| (state != IDLE).then_some(CANCELLING))
            .is_ok()
    }
}

impl SyncRun {
    /// Whether the user asked this run to stop.
    pub fn is_cancelled(&self) -> bool {
        self.state.load(Ordering::Acquire) == CANCELLING
    }
}

impl Drop for SyncRun {
    /// The record is closed before the gate opens: once it is open another run may begin, and
    /// closing then would close that run's record.
    fn drop(&mut self) {
        self.progress.end();
        self.state.store(IDLE, Ordering::Release);
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_gate.rs"]
mod tests;
