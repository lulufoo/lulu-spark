//! Workbench commands wrapping mcp_oauth ticket issue, rotate, and revoke.

use serde_json::{json, Value};

use crate::services::mcp_oauth::{
    issue_for_slot, rotate_for_slot, revoke_for_slot, OAuthError, Slot,
};

fn map_err(err: OAuthError) -> String {
    err.to_string()
}

#[tauri::command]
pub fn issue_cursor_ide_ticket() -> Result<Value, String> {
    let handle = issue_for_slot(Slot::CursorIde).map_err(map_err)?;
    Ok(json!({ "handle": handle.as_str() }))
}

#[tauri::command]
pub fn rotate_cursor_ide_ticket() -> Result<Value, String> {
    let handle = rotate_for_slot(Slot::CursorIde).map_err(map_err)?;
    Ok(json!({ "handle": handle.as_str() }))
}

#[tauri::command]
pub fn revoke_mcp_slot_ticket(slot: String) -> Result<Value, String> {
    let parsed = Slot::parse(&slot).map_err(map_err)?;
    revoke_for_slot(parsed).map_err(map_err)?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
#[path = "../unit-tests/commands/mcp_oauth.rs"]
mod tests;
