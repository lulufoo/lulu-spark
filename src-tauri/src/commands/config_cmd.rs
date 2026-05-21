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
    settings::apply_config_payload(&mut settings, &payload);
    settings::save(&settings).map_err(|e| format!("{e}"))?;
    Ok(settings::to_config_json(
        &settings,
        secrets::has_github_token(),
        secrets::has_meili_key(),
    ))
}

#[cfg(test)]
#[path = "../unit-tests/commands/config_cmd.rs"]
mod tests;
