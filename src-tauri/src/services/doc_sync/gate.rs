//! One sync at a time. Two runs at once would send and write the same documents twice.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The app's one gate. Clones share it.
#[derive(Debug, Clone, Default)]
pub struct SyncGate {
    running: Arc<AtomicBool>,
}

/// Held for as long as a run goes on. Dropping it, also when the run panics, opens the gate.
#[derive(Debug)]
pub struct SyncRun {
    running: Arc<AtomicBool>,
}

impl SyncGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// `None` when a run is going on.
    pub fn try_begin(&self) -> Option<SyncRun> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| SyncRun { running: Arc::clone(&self.running) })
    }
}

impl Drop for SyncRun {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_gate.rs"]
mod tests;
