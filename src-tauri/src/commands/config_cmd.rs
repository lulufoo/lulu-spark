//! Config read/write commands.
//!
//! Persists the Host/GLM `assistant_engine`, `llm.model`, and Host credential
//! (`api_key_host`). `get_config` / `to_config_json` expose only a key hint —
//! never plaintext credentials.
//! Illegal `assistant_engine` values are rejected at `apply_config_payload`.

use serde_json::Value;
use tauri::AppHandle;

use crate::config::secrets;
use crate::config::settings;

#[tauri::command]
pub fn set_config(_app: AppHandle, payload: Value) -> Result<Value, String> {
    apply_config_payload(&payload)
}

fn apply_config_payload(payload: &Value) -> Result<Value, String> {
    let mut settings = settings::load().map_err(|e| format!("{e}"))?;
    settings::apply_config_payload(&mut settings, payload).map_err(|e| format!("{e}"))?;
    if let Err(e) = secrets::apply_token_payload(payload) {
        return Ok(secrets::secret_error_json(&e));
    }
    if !settings::is_test_sandbox() {
        settings::normalize_cache_dir(&mut settings);
        settings::normalize_prod_paths(&mut settings);
    }
    settings::save(&settings).map_err(|e| format!("{e}"))?;
    Ok(settings::to_config_json(
        &settings,
        secrets::has_github_token(),
        secrets::has_meili_key(),
        secrets::has_host_key(),
    ))
}

#[cfg(test)]
#[path = "../unit-tests/commands/config_cmd.rs"]
mod tests;
