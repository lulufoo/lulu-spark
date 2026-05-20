//! Keychain secrets (macOS). Unit tests use an in-memory store (`cfg(test)`).

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{LazyLock, Mutex};

const SERVICE: &str = "lulu-workbench";

pub const KEY_GITHUB_TOKEN: &str = "github_token";
pub const KEY_MEILI_MASTER: &str = "meili_master_key";

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

pub fn get_secret(key: &str) -> Result<Option<String>, SecretError> {
    #[cfg(test)]
    {
        let map = test_store()?;
        return Ok(map.get(key).cloned());
    }
    #[cfg(not(test))]
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
    #[cfg(not(test))]
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
    #[cfg(not(test))]
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
    Ok(())
}

pub fn secret_error_json(err: &SecretError) -> serde_json::Value {
    serde_json::json!({
        "error": err.to_string(),
        "_status": 500
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_github_token() {
        test_secrets_clear();
        set_secret(KEY_GITHUB_TOKEN, "pat-test").expect("set");
        assert_eq!(
            get_secret(KEY_GITHUB_TOKEN).expect("get"),
            Some("pat-test".to_string())
        );
    }

    #[test]
    fn empty_github_token_deletes_entry() {
        test_secrets_clear();
        set_secret(KEY_GITHUB_TOKEN, "x").expect("set");
        delete_secret(KEY_GITHUB_TOKEN).expect("del");
        assert_eq!(get_secret(KEY_GITHUB_TOKEN).expect("get"), None);
    }
}
