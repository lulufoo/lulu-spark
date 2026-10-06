//! Host LLM credential. The store is the vault tree.

pub use crate::config::vault::{SecretError, KEY_LLM_API_KEY};

use crate::config::vault;

pub fn get_secret() -> Result<Option<String>, SecretError> {
    vault::get_llm_api_key()
}

pub fn set_secret(value: &str) -> Result<(), SecretError> {
    vault::set_llm_api_key(value)
}

/// Host credential present.
pub fn has_host_key() -> bool {
    get_secret()
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// Apply token fields from `set_config` payload.
pub fn apply_token_payload(payload: &serde_json::Value) -> Result<(), SecretError> {
    // Host key: empty does not clear (UI omits blank credentials).
    if let Some(v) = payload.get("api_key_host").and_then(|x| x.as_str()) {
        if !v.is_empty() {
            set_secret(v)?;
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

#[cfg(test)]
pub use tests::test_secrets_clear;
