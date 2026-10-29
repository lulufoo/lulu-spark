//! Spark commands wrapping mcp_oauth ticket issue, rotate, and revoke.

use serde_json::{json, Value};

use crate::services::mcp_oauth::{
    issue_for_slot, revoke_for_device, revoke_for_slot, rotate_for_slot, ticket_view, OAuthError,
    Slot, TicketHandle,
};

fn ide_slot(channel: &str) -> Result<Slot, String> {
    let parsed = Slot::parse(channel).map_err(|e| format!("{e}"))?;
    if parsed.is_ide() {
        Ok(parsed)
    } else {
        Err(OAuthError::rejected.to_string())
    }
}

fn ticket_payload(handle: &TicketHandle) -> Value {
    json!({ "handle": handle.as_str() })
}

#[tauri::command]
pub fn issue_cursor_ide_ticket(channel: String) -> Result<Value, String> {
    let slot = ide_slot(&channel)?;
    let handle = issue_for_slot(slot).map_err(|e| format!("{e}"))?;
    Ok(ticket_payload(&handle))
}

#[tauri::command]
pub fn rotate_cursor_ide_ticket(channel: String) -> Result<Value, String> {
    let slot = ide_slot(&channel)?;
    let handle = rotate_for_slot(slot).map_err(|e| format!("{e}"))?;
    Ok(ticket_payload(&handle))
}

#[tauri::command]
pub fn revoke_mcp_slot_ticket(slot: String) -> Result<Value, String> {
    let parsed = Slot::parse(&slot).map_err(|e| format!("{e}"))?;
    revoke_for_slot(parsed).map_err(|e| format!("{e}"))?;
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub fn get_mcp_ticket_view(channel: String) -> Result<Value, String> {
    ticket_view(&channel).map_err(|e| format!("{e}"))
}

#[tauri::command]
pub fn revoke_mcp_device_ticket(device_id: String) -> Result<Value, String> {
    revoke_for_device(&device_id).map_err(|e| format!("{e}"))?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
#[path = "../unit-tests/commands/mcp_oauth.rs"]
mod tests;
