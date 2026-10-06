//! Host secrets. The on-disk / Keychain shape is the vault tree in `vault.rs`.
//! `TestSandbox` (and `cfg(test)`) stay in memory. Debug builds persist the
//! same tree as nested TOML; release builds persist it as one Keychain item.

pub use crate::config::vault::{SecretError, KEY_LLM_API_KEY};

use crate::config::settings;
use crate::config::vault;

#[cfg(test)]
pub fn test_secrets_clear() {
    vault::test_clear_llm();
}

pub fn get_secret(key: &str) -> Result<Option<String>, SecretError> {
    let _ = settings::uses_in_memory_keychain();
    if key != KEY_LLM_API_KEY {
        return Ok(None);
    }
    vault::get_llm_api_key()
}

pub fn set_secret(key: &str, value: &str) -> Result<(), SecretError> {
    if key != KEY_LLM_API_KEY {
        return Ok(());
    }
    vault::set_llm_api_key(value)
}

pub fn delete_secret(key: &str) -> Result<(), SecretError> {
    if key != KEY_LLM_API_KEY {
        return Ok(());
    }
    vault::delete_llm_api_key()
}

/// Host credential present (`KEY_LLM_API_KEY`).
pub fn has_host_key() -> bool {
    get_secret(KEY_LLM_API_KEY)
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// Apply token fields from `set_config` payload.
pub fn apply_token_payload(payload: &serde_json::Value) -> Result<(), SecretError> {
    // Host key: empty does not clear (UI omits blank credentials).
    set_secret_if_nonempty(payload, "api_key_host", KEY_LLM_API_KEY)?;
    Ok(())
}

fn set_secret_if_nonempty(
    payload: &serde_json::Value,
    field: &str,
    key: &str,
) -> Result<(), SecretError> {
    if let Some(v) = payload.get(field).and_then(|x| x.as_str()) {
        if !v.is_empty() {
            set_secret(key, v)?;
        }
    }
    Ok(())
}

pub fn secret_error_json(err: &SecretError) -> serde_json::Value {
    serde_json::json!({
        "error": err.to_string(),
        "_status": 500
    })
}

#[cfg(test)]
#[path = "../unit-tests/config/secrets.rs"]
mod tests;
