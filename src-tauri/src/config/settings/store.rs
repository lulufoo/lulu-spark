use std::fs;
use std::path::PathBuf;

use super::llm::{
    llm_entry_by_type, normalize_engine_value, stamp_host_preset_on_load, stamp_readonly_preset_fields,
    upsert_llm_entry,
};
use super::sandbox::{
    is_unstable_path, normalize_prod_paths, should_normalize_for_path, validate_sandbox_against_prod,
};
use super::types::{
    default_cache_dir, home_dir, is_test_sandbox, test_sandbox_id_raw, validate_test_sandbox_id,
    AppSettings, LlmSettings, LlmSettingsEntry, SettingsError, PROD_CONFIG_FILE_NAME,
};

/// Fixed prod config directory (never follows `TestSandbox`).
pub fn prod_config_dir() -> PathBuf {
    home_dir().join(".config").join("lulu-spark")
}

/// Shared sandbox config directory (secrets + template `config.toml`).
pub fn shared_sandbox_config_dir() -> PathBuf {
    home_dir().join(".config").join("lulu-spark-sandbox")
}

/// Active config directory from `TestSandbox` / `TestSandboxId`.
///
/// Invalid `TestSandboxId` returns `Err` — callers must fail closed.
pub fn settings_config_dir() -> Result<PathBuf, SettingsError> {
    if !is_test_sandbox() {
        return Ok(prod_config_dir());
    }
    match test_sandbox_id_raw() {
        None => Ok(shared_sandbox_config_dir()),
        Some(id) => {
            validate_test_sandbox_id(&id)?;
            Ok(home_dir()
                .join(".config")
                .join(format!("lulu-spark-sandbox-{id}")))
        }
    }
}

pub fn prod_config_file_path() -> PathBuf {
    prod_config_dir().join(PROD_CONFIG_FILE_NAME)
}

pub fn config_file_path() -> Result<PathBuf, SettingsError> {
    Ok(settings_config_dir()?.join(PROD_CONFIG_FILE_NAME))
}

/// Read prod `config.toml` only (for sandbox prod-path guards).
pub fn load_prod_settings() -> AppSettings {
    let path = prod_config_file_path();
    if !path.is_file() {
        return AppSettings::default();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return AppSettings::default();
    };
    let Ok(mut settings) = toml::from_str::<AppSettings>(&text) else {
        return AppSettings::default();
    };
    // Temp HOME fixtures (unit tests) keep explicit roots; do not heal to ~/Code.
    if !path.ancestors().any(is_unstable_path) {
        normalize_prod_paths(&mut settings);
    }
    settings
}

pub fn load() -> Result<AppSettings, SettingsError> {
    let path = config_file_path()?;
    if is_test_sandbox() && !path.is_file() {
        return Err(SettingsError::ConfigGuard(format!(
            "sandbox config missing (initiator must fork): {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(&path)?;
    let mut settings: AppSettings = toml::from_str(&text)?;
    if should_normalize_for_path(&path) {
        normalize_prod_paths(&mut settings);
    }
    stamp_host_preset_on_load(&mut settings);
    if is_test_sandbox() {
        validate_sandbox_against_prod(&settings, &load_prod_settings())?;
    }
    Ok(settings)
}

pub fn save(settings: &AppSettings) -> Result<(), SettingsError> {
    let path = config_file_path()?;
    let mut to_save = settings.clone();
    if should_normalize_for_path(&path) {
        normalize_prod_paths(&mut to_save);
    }
    if is_test_sandbox() {
        validate_sandbox_against_prod(&to_save, &load_prod_settings())?;
    }
    let text = toml::to_string_pretty(&to_save)?;

    #[cfg(test)]
    {
        super::sandbox::reject_test_write_to_machine_config(&path)?;
        super::sandbox::atomic_write_test_config(&path, &text)
    }

    #[cfg(not(test))]
    {
        let dir = settings_config_dir()?;
        fs::create_dir_all(&dir)?;
        fs::write(path, text)?;
        Ok(())
    }
}

/// JSON shape for `get_config` / `set_config` (frontend field names).
/// Never echoes plaintext api keys — only the Host/GLM key hint.
pub fn to_config_json(
    settings: &AppSettings,
    has_host_key: bool,
) -> serde_json::Value {
    let current = if let Some(engine) = normalize_engine_value(&settings.assistant_engine) {
        llm_entry_by_type(&settings.llm, engine)
            .map(LlmSettingsEntry::fields)
            .unwrap_or_default()
    } else {
        LlmSettings::default()
    };
    serde_json::json!({
        "spark_root": settings.spark_root.to_string_lossy(),
        "knowledge_root": default_cache_dir().join("data").join("knowledge").to_string_lossy(),
        "notes_root": default_cache_dir().join("data").join("notes").to_string_lossy(),
        "cache_dir": settings.cache_dir.to_string_lossy(),
        "assistant_engine": settings.assistant_engine,
        "http_port": settings.effective_http_port(),
        "mcp_port": settings.effective_mcp_port(),
        "gateway_port": settings.effective_gateway_port(),
        "test_sandbox": is_test_sandbox(),
        "has_host_key": has_host_key,
        "llm": {
            "platform": current.platform,
            "base_url": current.base_url,
            "model": current.model,
        },
    })
}

/// Apply `set_config` payload keys onto settings (toml fields only).
/// Illegal or legacy `assistant_engine` values are rejected.
/// Flat `llm` maps to the active category list entry (create if missing).
/// `llm.platform` from the client is ignored (preset readonly). `llm.base_url` is
/// writable; empty falls back to that category's default. Platform is stamped
/// whenever `assistant_engine`, `llm.model`, or `llm.base_url` is applied.
pub fn apply_config_payload(
    settings: &mut AppSettings,
    payload: &serde_json::Value,
) -> Result<(), SettingsError> {
    let mut engine_touched = false;
    let mut llm_fields_touched = false;
    if let Some(v) = payload.get("assistant_engine").and_then(|x| x.as_str()) {
        let Some(normalized) = normalize_engine_value(v) else {
            return Err(SettingsError::ConfigGuard(format!(
                "invalid assistant_engine value: {v}"
            )));
        };
        settings.assistant_engine = normalized.to_string();
        engine_touched = true;
    }
    if let Some(v) = payload.get("spark_root").and_then(|x| x.as_str()) {
        settings.spark_root = std::path::PathBuf::from(v);
    }
    if let Some(v) = payload.get("knowledge_root").and_then(|x| x.as_str()) {
        settings.knowledge_root = std::path::PathBuf::from(v);
    }
    if let Some(llm) = payload.get("llm").and_then(|x| x.as_object()) {
        let model = llm.get("model").and_then(|x| x.as_str());
        let base_url = llm.get("base_url").and_then(|x| x.as_str());
        if model.is_some() || base_url.is_some() {
            let engine = normalize_engine_value(&settings.assistant_engine).unwrap_or("host");
            let mut fields = llm_entry_by_type(&settings.llm, engine)
                .map(LlmSettingsEntry::fields)
                .unwrap_or_default();
            if let Some(v) = model {
                fields.model = v.to_string();
            }
            if let Some(v) = base_url {
                fields.base_url = v.trim().to_string();
            }
            upsert_llm_entry(&mut settings.llm, engine, &fields)?;
            llm_fields_touched = true;
        }
    }
    if engine_touched || llm_fields_touched {
        stamp_readonly_preset_fields(settings);
    }
    // `cache_dir` is not user-settable via API; use `default_cache_dir()` / manual toml edit.
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/config/settings/store.rs"]
mod tests;
