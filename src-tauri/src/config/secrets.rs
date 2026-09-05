//! Keychain secrets (macOS). `TestSandbox` (and `cfg(test)`) use an in-memory
//! store. Debug (dev) builds without a sandbox use a plain-text TOML file to
//! avoid Keychain prompts on every hot-rebuild; release builds without a
//! sandbox use the system Keychain.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::config::settings;

#[allow(dead_code)]
fn keyring_service() -> &'static str {
    "lulu-workbench"
}

pub const KEY_GITHUB_TOKEN: &str = "github_token";
/// Host/GLM credential slot.
pub const KEY_LLM_API_KEY: &str = "llm_api_key";

#[derive(Debug)]
pub enum SecretError {
    Keyring(String),
    Poisoned,
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::Keyring(e) => write!(f, "keyring: {e}"),
            SecretError::Poisoned => write!(f, "lock poisoned"),
        }
    }
}

// ── memory store (`TestSandbox` / `cfg(test)`) ────────────────────────────────

static MEMORY_SECRETS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub fn test_secrets_clear() {
    if let Ok(mut m) = MEMORY_SECRETS.lock() {
        m.clear();
    }
}

fn memory_store() -> Result<std::sync::MutexGuard<'static, HashMap<String, String>>, SecretError> {
    MEMORY_SECRETS.lock().map_err(|_| SecretError::Poisoned)
}

// ── dev store (debug builds only) ─────────────────────────────────────────────

#[cfg(all(not(test), debug_assertions))]
fn dev_secrets_path() -> std::path::PathBuf {
    settings::prod_config_dir().join("dev-secrets.toml")
}

#[cfg(all(not(test), debug_assertions))]
fn read_dev_secrets() -> std::collections::HashMap<String, String> {
    let path = dev_secrets_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return std::collections::HashMap::new();
    };
    toml::from_str(&content).unwrap_or_default()
}

#[cfg(all(not(test), debug_assertions))]
fn write_dev_secrets(
    map: &std::collections::HashMap<String, String>,
) -> Result<(), SecretError> {
    let path = dev_secrets_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SecretError::Keyring(e.to_string()))?;
    }
    let content = toml::to_string(map).map_err(|e| SecretError::Keyring(e.to_string()))?;
    std::fs::write(&path, content).map_err(|e| SecretError::Keyring(e.to_string()))
}

// ── public API ────────────────────────────────────────────────────────────────

pub fn get_secret(key: &str) -> Result<Option<String>, SecretError> {
    if settings::uses_in_memory_keychain() {
        let map = memory_store()?;
        return Ok(map.get(key).cloned());
    }
    #[cfg(not(test))]
    {
        return persistent_get_secret(key);
    }
    #[cfg(test)]
    {
        let map = memory_store()?;
        Ok(map.get(key).cloned())
    }
}

pub fn set_secret(key: &str, value: &str) -> Result<(), SecretError> {
    if settings::uses_in_memory_keychain() {
        let mut map = memory_store()?;
        map.insert(key.to_string(), value.to_string());
        return Ok(());
    }
    #[cfg(not(test))]
    {
        return persistent_set_secret(key, value);
    }
    #[cfg(test)]
    {
        let mut map = memory_store()?;
        map.insert(key.to_string(), value.to_string());
        Ok(())
    }
}

pub fn delete_secret(key: &str) -> Result<(), SecretError> {
    if settings::uses_in_memory_keychain() {
        let mut map = memory_store()?;
        map.remove(key);
        return Ok(());
    }
    #[cfg(not(test))]
    {
        return persistent_delete_secret(key);
    }
    #[cfg(test)]
    {
        let mut map = memory_store()?;
        map.remove(key);
        Ok(())
    }
}

#[cfg(all(not(test), debug_assertions))]
fn persistent_get_secret(key: &str) -> Result<Option<String>, SecretError> {
    let map = read_dev_secrets();
    Ok(map.get(key).cloned())
}

#[cfg(all(not(test), debug_assertions))]
fn persistent_set_secret(key: &str, value: &str) -> Result<(), SecretError> {
    let mut map = read_dev_secrets();
    map.insert(key.to_string(), value.to_string());
    write_dev_secrets(&map)
}

#[cfg(all(not(test), debug_assertions))]
fn persistent_delete_secret(key: &str) -> Result<(), SecretError> {
    let mut map = read_dev_secrets();
    map.remove(key);
    write_dev_secrets(&map)
}

#[cfg(all(not(test), not(debug_assertions)))]
fn persistent_get_secret(key: &str) -> Result<Option<String>, SecretError> {
    let entry = keyring::Entry::new(keyring_service(), key)
        .map_err(|e| SecretError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(SecretError::Keyring(e.to_string())),
    }
}

#[cfg(all(not(test), not(debug_assertions)))]
fn persistent_set_secret(key: &str, value: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new(keyring_service(), key)
        .map_err(|e| SecretError::Keyring(e.to_string()))?;
    entry
        .set_password(value)
        .map_err(|e| SecretError::Keyring(e.to_string()))
}

#[cfg(all(not(test), not(debug_assertions)))]
fn persistent_delete_secret(key: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new(keyring_service(), key)
        .map_err(|e| SecretError::Keyring(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(SecretError::Keyring(e.to_string())),
    }
}

pub fn has_github_token() -> bool {
    get_secret(KEY_GITHUB_TOKEN)
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
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
    if let Some(v) = payload.get("github_token").and_then(|x| x.as_str()) {
        if v.is_empty() {
            delete_secret(KEY_GITHUB_TOKEN)?;
        } else {
            set_secret(KEY_GITHUB_TOKEN, v)?;
        }
    }
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
