//! Stage-entry approval for outside writes. Does not talk to Tauri.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::agent::approval::{self, ApprovalAsk, ApprovalChoice};
use crate::agent::fence::{stored_path, validate_stage_file, PathFence};
use crate::agent::hard_deny::HardDenyList;
use crate::agent::permission::{
    decide, load_records, PermissionDecision, PermissionKind, PermissionRequest,
};
use crate::agent::session::{self, Session};
use crate::services::id::random_entry_id;

#[derive(Debug)]
pub enum GateError {
    Denied(String),
    Cancelled,
}

pub fn prepare_outside_write(
    path: &str,
    session: &mut Session,
    fence: &PathFence,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<PathBuf, GateError> {
    let canon = validate_stage_file(path, fence).map_err(GateError::Denied)?;
    if HardDenyList::spark_defaults().is_some_and(|list| list.contains(&canon)) {
        return Err(GateError::Denied(
            "path is denied by the hard-deny list".into(),
        ));
    }
    if fence.allows_write(&canon) {
        return Ok(canon);
    }
    match decide_path(&canon, &session.session_id, fence) {
        PermissionDecision::Deny => {
            return Err(GateError::Denied(
                "path is denied by the hard-deny list".into(),
            ));
        }
        PermissionDecision::Allow => {}
        PermissionDecision::Ask => {
            let choice = wait_for_choice(&canon, &session.session_id, is_cancelled)?;
            match choice {
                ApprovalChoice::Deny => {
                    return Err(GateError::Denied("approval denied".into()));
                }
                ApprovalChoice::AllowOnce | ApprovalChoice::AllowSession => {}
            }
        }
    }
    session
        .register_staged(&canon.to_string_lossy(), None, None)
        .map_err(GateError::Denied)?;
    let _ = session::save_session(session);
    Ok(canon)
}

fn decide_path(path: &Path, session_id: &str, fence: &PathFence) -> PermissionDecision {
    let records = session::session_file_path(session_id)
        .ok()
        .and_then(|db| load_records(&db).ok())
        .unwrap_or_default();
    decide(
        &PermissionRequest {
            kind: PermissionKind::Fs,
            path: path.to_path_buf(),
            session_id: session_id.to_string(),
            request_id: String::new(),
        },
        fence,
        &records,
    )
}

fn wait_for_choice(
    path: &Path,
    session_id: &str,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<ApprovalChoice, GateError> {
    let request_id = format!("apr_{}", random_entry_id());
    let ask = ApprovalAsk {
        request_id: request_id.clone(),
        kind: "fs".into(),
        path: stored_path(path.to_path_buf())
            .to_string_lossy()
            .into_owned(),
        session_id: session_id.to_string(),
    };
    approval::emit_ask(&ask);
    let wait_id = request_id.clone();
    let wait_session = session_id.to_string();
    let handle = thread::spawn(move || approval::create(&wait_id, &wait_session));
    loop {
        if handle.is_finished() {
            return handle
                .join()
                .map_err(|_| GateError::Denied("approval wait crashed".into()));
        }
        if is_cancelled() {
            approval::cancel(&request_id);
            return Err(GateError::Cancelled);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_write_gate_tests.rs"]
mod tests;
