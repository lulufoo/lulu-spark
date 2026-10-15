//! LLM credentials, one per engine. The store is the vault tree.

pub use crate::config::vault::SecretError;

use crate::config::settings::{normalize_engine_value, LLM_ENGINES};
use crate::config::vault;

fn resolve_engine_id(engine: &str) -> Result<&'static str, SecretError> {
    normalize_engine_value(engine).ok_or_else(|| SecretError::InvalidEngine(engine.to_string()))
}

pub fn get_secret(engine: &str) -> Result<Option<String>, SecretError> {
    vault::get_llm_api_key(resolve_engine_id(engine)?)
}

pub fn set_secret(engine: &str, value: &str) -> Result<(), SecretError> {
    vault::set_llm_api_key(resolve_engine_id(engine)?, value)
}

/// Engines that currently have a stored credential. Read failure → none.
pub fn list_saved_engines() -> Vec<&'static str> {
    let Ok(vault) = vault::read_vault() else {
        return Vec::new();
    };
    LLM_ENGINES
        .iter()
        .copied()
        .filter(|engine| vault.llm_api_key(engine).is_some())
        .collect()
}

/// Apply `llm_api_key: { engine, value }` from the `set_config` payload.
/// An empty `value` does not clear (UI omits blank credentials). An unknown or
/// missing `engine` is rejected.
pub fn apply_token_payload(payload: &serde_json::Value) -> Result<(), SecretError> {
    let Some(slot) = payload.get("llm_api_key").filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let engine = slot
        .get("engine")
        .and_then(|x| x.as_str())
        .ok_or_else(|| SecretError::InvalidEngine("<missing>".to_string()))?;
    let engine = resolve_engine_id(engine)?;
    let value = slot.get("value").and_then(|x| x.as_str()).unwrap_or("");
    if !value.is_empty() {
        set_secret(engine, value)?;
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
