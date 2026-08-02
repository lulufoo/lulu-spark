//! Keychain secrets (macOS). Unit tests use an in-memory store (`cfg(test)`).
//! Debug (dev) builds use a plain-text TOML file to avoid Keychain prompts on
//! every hot-rebuild; release builds use the system Keychain.

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{LazyLock, Mutex};

const SERVICE: &str = "lulu-workbench";

pub const KEY_GITHUB_TOKEN: &str = "github_token";
pub const KEY_MEILI_MASTER: &str = "meili_master_key";
/// Host-engine credential slot (legacy `api_key` / `KEY_LLM_API_KEY`).
pub const KEY_LLM_API_KEY: &str = "llm_api_key";
/// Cursor-engine credential slot (`api_key_cursor`).
pub const KEY_LLM_API_KEY_CURSOR: &str = "llm_api_key_cursor";

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

// ── test store ────────────────────────────────────────────────────────────────

#[cfg(test)]
static TEST_SECRETS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub fn test_secrets_clear() {
    if let Ok(mut m) = TEST_SECRETS.lock() {
        m.clear();
    }
}

#[cfg(test)]
fn test_store() -> Result<std::sync::MutexGuard<'static, HashMap<String, String>>, SecretError> {
    TEST_SECRETS.lock().map_err(|_| SecretError::Poisoned)
}

// ── dev store (debug builds only) ─────────────────────────────────────────────

#[cfg(all(not(test), debug_assertions))]
fn dev_secrets_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("lulu-workbench")
        .join("dev-secrets.toml")
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
    #[cfg(test)]
    {
        let map = test_store()?;
        return Ok(map.get(key).cloned());
    }
    #[cfg(all(not(test), debug_assertions))]
    {
        let map = read_dev_secrets();
        return Ok(map.get(key).cloned());
    }
    #[cfg(all(not(test), not(debug_assertions)))]
    {
        let entry = keyring::Entry::new(SERVICE, key).map_err(|e| SecretError::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError::Keyring(e.to_string())),
        }
    }
}

pub fn set_secret(key: &str, value: &str) -> Result<(), SecretError> {
    #[cfg(test)]
    {
        let mut map = test_store()?;
        map.insert(key.to_string(), value.to_string());
        return Ok(());
    }
    #[cfg(all(not(test), debug_assertions))]
    {
        let mut map = read_dev_secrets();
        map.insert(key.to_string(), value.to_string());
        return write_dev_secrets(&map);
    }
    #[cfg(all(not(test), not(debug_assertions)))]
    {
        let entry = keyring::Entry::new(SERVICE, key).map_err(|e| SecretError::Keyring(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| SecretError::Keyring(e.to_string()))
    }
}

pub fn delete_secret(key: &str) -> Result<(), SecretError> {
    #[cfg(test)]
    {
        let mut map = test_store()?;
        map.remove(key);
        return Ok(());
    }
    #[cfg(all(not(test), debug_assertions))]
    {
        let mut map = read_dev_secrets();
        map.remove(key);
        return write_dev_secrets(&map);
    }
    #[cfg(all(not(test), not(debug_assertions)))]
    {
        let entry = keyring::Entry::new(SERVICE, key).map_err(|e| SecretError::Keyring(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError::Keyring(e.to_string())),
        }
    }
}

pub fn has_github_token() -> bool {
    get_secret(KEY_GITHUB_TOKEN)
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn has_meili_key() -> bool {
    get_secret(KEY_MEILI_MASTER)
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// Host credential present (`KEY_LLM_API_KEY`). Alias kept as `has_llm_key`.
pub fn has_host_key() -> bool {
    get_secret(KEY_LLM_API_KEY)
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn has_cursor_key() -> bool {
    get_secret(KEY_LLM_API_KEY_CURSOR)
        .ok()
        .flatten()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// Backward-compatible alias: host credential slot.
pub fn has_llm_key() -> bool {
    has_host_key()
}

/// Ensure legacy `KEY_LLM_API_KEY` remains the host slot (never silently discarded).
pub fn migrate_legacy_llm_api_key_to_host() -> Result<(), SecretError> {
    // Host slot *is* `KEY_LLM_API_KEY`; migration is a no-op preserve check.
    let _ = get_secret(KEY_LLM_API_KEY)?;
    Ok(())
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
    if let Some(v) = payload.get("meili_master_key").and_then(|x| x.as_str()) {
        if v.is_empty() {
            delete_secret(KEY_MEILI_MASTER)?;
        } else {
            set_secret(KEY_MEILI_MASTER, v)?;
        }
    }
    // Legacy `api_key`: empty clears (existing set_config contract).
    if let Some(v) = payload.get("api_key").and_then(|x| x.as_str()) {
        if v.is_empty() {
            delete_secret(KEY_LLM_API_KEY)?;
        } else {
            set_secret(KEY_LLM_API_KEY, v)?;
        }
    }
    // Per-category keys: empty does not clear (UI omits blank credentials).
    set_secret_if_nonempty(payload, "api_key_host", KEY_LLM_API_KEY)?;
    set_secret_if_nonempty(payload, "api_key_cursor", KEY_LLM_API_KEY_CURSOR)?;
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
