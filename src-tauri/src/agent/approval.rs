//! Approval wait/resolve. No Tauri, no UI. Timeout and cancel are deny.

use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::agent::permission;
use crate::agent::session;

pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalChoice {
    AllowOnce,
    AllowSession,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalAsk {
    pub request_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub path: String,
    pub session_id: String,
}

type ApprovalEmit = Arc<dyn Fn(&ApprovalAsk) + Send + Sync>;

fn emitter() -> &'static Mutex<Option<ApprovalEmit>> {
    static EMITTER: OnceLock<Mutex<Option<ApprovalEmit>>> = OnceLock::new();
    EMITTER.get_or_init(|| Mutex::new(None))
}

pub fn set_emitter(next: Option<ApprovalEmit>) {
    *emitter().lock().unwrap_or_else(|e| e.into_inner()) = next;
}

pub fn emit_ask(ask: &ApprovalAsk) {
    if let Some(emit) = emitter().lock().unwrap_or_else(|e| e.into_inner()).clone() {
        emit(ask);
    }
}

struct Pending {
    session_id: String,
    tx: mpsc::Sender<ApprovalChoice>,
}

fn registry() -> &'static Mutex<HashMap<String, Pending>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Pending>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Block until `resolve` for this id, or until the timeout. Missing/cancel/timeout = deny.
pub fn create(request_id: &str, session_id: &str) -> ApprovalChoice {
    create_for(request_id, session_id, APPROVAL_TIMEOUT)
}

pub fn create_for(request_id: &str, session_id: &str, timeout: Duration) -> ApprovalChoice {
    let id = request_id.trim();
    let session_id = session_id.trim();
    if id.is_empty() || session_id.is_empty() {
        return ApprovalChoice::Deny;
    }
    let (tx, rx) = mpsc::channel();
    {
        let mut map = registry().lock().unwrap_or_else(|e| e.into_inner());
        if map.contains_key(id) {
            return ApprovalChoice::Deny;
        }
        map.insert(
            id.to_string(),
            Pending {
                session_id: session_id.to_string(),
                tx,
            },
        );
    }
    match rx.recv_timeout(timeout) {
        Ok(choice) => choice,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            if take(id).is_some() {
                ApprovalChoice::Deny
            } else {
                rx.recv().unwrap_or(ApprovalChoice::Deny)
            }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => ApprovalChoice::Deny,
    }
}

/// Apply a choice to the waiter with this id. Unknown id is deny and does not touch others.
pub fn resolve(request_id: &str, choice: ApprovalChoice) -> ApprovalChoice {
    let Some(pending) = take(request_id.trim()) else {
        return ApprovalChoice::Deny;
    };
    let applied = apply(choice, &pending.session_id);
    let _ = pending.tx.send(applied);
    applied
}

/// Turn cancel for one request id. Same as resolve deny.
pub fn cancel(request_id: &str) -> ApprovalChoice {
    resolve(request_id, ApprovalChoice::Deny)
}

fn apply(choice: ApprovalChoice, session_id: &str) -> ApprovalChoice {
    match choice {
        ApprovalChoice::AllowSession => match session::session_file_path(session_id)
            .and_then(|path| permission::store_session_allow(&path, session_id))
        {
            Ok(()) => ApprovalChoice::AllowSession,
            Err(_) => ApprovalChoice::Deny,
        },
        other => other,
    }
}

fn take(request_id: &str) -> Option<Pending> {
    if request_id.is_empty() {
        return None;
    }
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(request_id)
}

#[cfg(test)]
pub fn reset_for_tests() {
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

pub fn is_pending(request_id: &str) -> bool {
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains_key(request_id)
}

pub fn pending_ids() -> Vec<String> {
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .keys()
        .cloned()
        .collect()
}

pub fn deny_all_pending() {
    for id in pending_ids() {
        resolve(&id, ApprovalChoice::Deny);
    }
}

#[cfg(test)]
#[path = "../unit-tests/agent/approval_tests.rs"]
mod tests;
