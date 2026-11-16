//! Config read/write commands.
//!
//! Persists `assistant_engine`, `llm.model` / `llm.base_url`, and the per-engine
//! credential (`llm_api_key: { engine, value }`). `get_config` / `to_config_json`
//! expose only a per-engine `key_saved` flag — never plaintext credentials.
//! Illegal `assistant_engine` values are rejected at `apply_config_payload`.

use serde_json::Value;
use tauri::AppHandle;

use crate::config::secrets;
use crate::config::settings;

#[tauri::command]
pub fn set_config(_app: AppHandle, payload: Value) -> Result<Value, String> {
    apply_config_payload(&payload)
}

static APPLY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn apply_config_payload(payload: &Value) -> Result<Value, String> {
    // One config apply at a time: two concurrent saves could otherwise
    // interleave load→save and silently drop each other's fields.
    let _apply = APPLY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut settings = settings::load().map_err(|e| format!("{e}"))?;
    settings::apply_config_payload(&mut settings, payload).map_err(|e| format!("{e}"))?;
    if let Err(e) = secrets::apply_token_payload(payload) {
        return Ok(secrets::secret_error_json(&e));
    }
    if settings::runtime_normalize_prod_paths().map_err(|e| format!("{e}"))? {
        settings::normalize_cache_dir(&mut settings);
        settings::normalize_prod_paths(&mut settings);
    }
    settings::save(&settings).map_err(|e| format!("{e}"))?;
    // New requests pick up the saved network settings without a restart.
    crate::net::refresh_now();
    settings::to_config_json(&settings, &secrets::list_saved_engines()).map_err(|e| format!("{e}"))
}

#[cfg(test)]
#[path = "../unit-tests/commands/config_cmd.rs"]
mod tests;
