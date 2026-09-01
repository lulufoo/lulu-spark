use std::sync::{Mutex, OnceLock};

use super::types::AIAssistantSession;

fn live_slot() -> &'static Mutex<AIAssistantSession> {
    static LIVE: OnceLock<Mutex<AIAssistantSession>> = OnceLock::new();
    LIVE.get_or_init(|| Mutex::new(AIAssistantSession::default()))
}

/// Clone of the sole live context owner (binding / MCP / session_id / cancel).
pub fn live_context_owner() -> AIAssistantSession {
    live_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

pub(crate) fn with_live_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut AIAssistantSession) -> R,
{
    let mut guard = live_slot().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

pub fn reset_live_for_tests() {
    *live_slot().lock().unwrap_or_else(|e| e.into_inner()) = AIAssistantSession::default();
}
