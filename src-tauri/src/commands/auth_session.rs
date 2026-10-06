use tauri::{AppHandle, Emitter};

use crate::config::vault::AuthSession;

pub const AUTH_SCHEME_OPENED_EVENT: &str = "spark-scheme:opened";

#[tauri::command]
pub fn get_auth_session(_app: AppHandle) -> Result<Option<AuthSession>, String> {
    crate::config::vault::get_auth_session().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_auth_session(_app: AppHandle, session: AuthSession) -> Result<(), String> {
    crate::config::vault::set_auth_session(&session).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_auth_session(_app: AppHandle) -> Result<(), String> {
    crate::config::vault::delete_auth_session().map_err(|e| e.to_string())
}

pub fn emit_opened_scheme(app: &AppHandle, url: &str) {
    let _ = app.emit(
        AUTH_SCHEME_OPENED_EVENT,
        serde_json::json!({ "scheme": url }),
    );
}
