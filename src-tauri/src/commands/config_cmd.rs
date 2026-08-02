//! Config read/write commands.
//!
//! `assistant_engine` (`host` | `cursor`) is persisted via settings and exposed
//! on the existing `to_config_json` / `apply_config_payload` read path for
//! engine routing (UI controls for this field are out of this slice).

use serde_json::Value;
use tauri::AppHandle;

use crate::config::secrets;
use crate::config::settings;

#[tauri::command]
pub fn set_config(_app: AppHandle, payload: Value) -> Result<Value, String> {
    if let Err(e) = secrets::apply_token_payload(&payload) {
        return Ok(secrets::secret_error_json(&e));
    }
    let mut settings = settings::load().map_err(|e| format!("{e}"))?;
    settings::apply_config_payload(&mut settings, &payload).map_err(|e| format!("{e}"))?;
    if !settings::uses_dev_config() {
        settings::normalize_cache_dir(&mut settings);
        settings::normalize_prod_paths(&mut settings);
    }
    settings::save(&settings).map_err(|e| format!("{e}"))?;
    Ok(settings::to_config_json(
        &settings,
        secrets::has_github_token(),
        secrets::has_meili_key(),
        secrets::has_host_key(),
        secrets::has_cursor_key(),
    ))
}

#[cfg(test)]
#[path = "../unit-tests/commands/config_cmd.rs"]
mod tests;
