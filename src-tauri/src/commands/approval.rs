//! Approval IPC: emit a request, invoke the same id back. No dialog.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::agent::approval::{self, ApprovalAsk, ApprovalChoice};

pub const EVENT_APPROVAL_REQUESTED: &str = "ai-assistant:approval-requested";

pub type ApprovalRequestPayload = ApprovalAsk;

pub fn approval_request_payload(
    request_id: &str,
    path: &str,
    session_id: &str,
) -> ApprovalRequestPayload {
    ApprovalAsk {
        request_id: request_id.to_string(),
        kind: "fs".into(),
        path: path.to_string(),
        session_id: session_id.to_string(),
    }
}

pub fn emit_approval_request(app: &AppHandle, payload: &ApprovalRequestPayload) {
    let _ = app.emit(EVENT_APPROVAL_REQUESTED, payload);
}

pub fn install_emitter(app: AppHandle) {
    approval::set_emitter(Some(std::sync::Arc::new(move |ask| {
        emit_approval_request(&app, ask);
    })));
}

pub fn resolve_approval_json(request_id: &str, choice: &str) -> Result<Value, String> {
    let id = request_id.trim();
    if id.is_empty() || !approval::is_pending(id) {
        return Err("approval request not found".into());
    }
    let parsed = parse_choice(choice)?;
    let applied = approval::resolve(id, parsed);
    Ok(json!({
        "ok": true,
        "choice": choice_name(applied),
    }))
}

fn parse_choice(choice: &str) -> Result<ApprovalChoice, String> {
    match choice.trim() {
        "allow_once" => Ok(ApprovalChoice::AllowOnce),
        "allow_session" => Ok(ApprovalChoice::AllowSession),
        "deny" => Ok(ApprovalChoice::Deny),
        _ => Err("invalid approval choice".into()),
    }
}

fn choice_name(choice: ApprovalChoice) -> &'static str {
    match choice {
        ApprovalChoice::AllowOnce => "allow_once",
        ApprovalChoice::AllowSession => "allow_session",
        ApprovalChoice::Deny => "deny",
    }
}

#[tauri::command]
pub async fn resolve_approval(request_id: String, choice: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || resolve_approval_json(&request_id, &choice))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../unit-tests/commands/approval.rs"]
mod tests;
