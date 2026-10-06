use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::config::settings;

#[cfg(not(debug_assertions))]
use super::codec::{vault_from_json, vault_to_json};
#[cfg(debug_assertions)]
use super::codec::{vault_from_toml, vault_to_toml};
use super::types::{SecretError, Vault};

pub const KEYCHAIN_SERVICE: &str = "lulu-spark";
pub const ACCOUNT_VAULT: &str = "vault";

pub(super) fn io_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub(super) fn scope_id() -> String {
    if cfg!(test) {
        return "_default".to_string();
    }
    if settings::is_test_sandbox() {
        settings::test_sandbox_id_raw().unwrap_or_else(|| "_shared_sandbox".to_string())
    } else {
        "_default".to_string()
    }
}

fn memory_map() -> std::sync::MutexGuard<'static, HashMap<String, Vault>> {
    static MEMORY: OnceLock<Mutex<HashMap<String, Vault>>> = OnceLock::new();
    MEMORY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub(super) fn memory_fail_map() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
    static FAIL: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    FAIL.get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn memory_fail() -> Option<String> {
    memory_fail_map().get(&scope_id()).cloned()
}

pub(super) fn memory_vault() -> Vault {
    memory_map()
        .get(&scope_id())
        .cloned()
        .unwrap_or_default()
}

pub(super) fn store_memory_vault(vault: &Vault) {
    memory_map().insert(scope_id(), vault.clone());
}

fn load_raw() -> Result<Vault, SecretError> {
    if settings::uses_in_memory_keychain() {
        if let Some(e) = memory_fail() {
            return Err(SecretError::Store(e));
        }
        Ok(memory_vault())
    } else {
        persistent_load()
    }
}

fn store_raw(vault: &Vault) -> Result<(), SecretError> {
    if settings::uses_in_memory_keychain() {
        if let Some(e) = memory_fail() {
            return Err(SecretError::Store(e));
        }
        store_memory_vault(vault);
        Ok(())
    } else {
        persistent_store(vault)
    }
}

#[cfg(debug_assertions)]
fn persistent_load() -> Result<Vault, SecretError> {
    let path = settings::prod_config_dir().join("dev-secrets.toml");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Ok(Vault::default());
    };
    vault_from_toml(&raw)
}

#[cfg(debug_assertions)]
fn persistent_store(vault: &Vault) -> Result<(), SecretError> {
    let path = settings::prod_config_dir().join("dev-secrets.toml");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SecretError::Store(e.to_string()))?;
    }
    let content = vault_to_toml(vault)?;
    std::fs::write(path, content).map_err(|e| SecretError::Store(e.to_string()))
}

#[cfg(not(debug_assertions))]
fn persistent_load() -> Result<Vault, SecretError> {
    match keychain_get(KEYCHAIN_SERVICE, ACCOUNT_VAULT)? {
        Some(raw) => vault_from_json(&raw),
        None => Ok(Vault::default()),
    }
}

#[cfg(not(debug_assertions))]
fn persistent_store(vault: &Vault) -> Result<(), SecretError> {
    keychain_set(KEYCHAIN_SERVICE, ACCOUNT_VAULT, &vault_to_json(vault)?)
}

#[cfg(not(debug_assertions))]
fn keychain_get(service: &str, account: &str) -> Result<Option<String>, SecretError> {
    let entry = keyring::Entry::new(service, account)
        .map_err(|e| SecretError::Store(e.to_string()))?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(SecretError::Store(e.to_string())),
    }
}

#[cfg(not(debug_assertions))]
fn keychain_set(service: &str, account: &str, value: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new(service, account)
        .map_err(|e| SecretError::Store(e.to_string()))?;
    entry
        .set_password(value)
        .map_err(|e| SecretError::Store(e.to_string()))
}

pub fn read_vault() -> Result<Vault, SecretError> {
    let _guard = io_lock();
    load_raw()
}

pub fn update_vault<F>(edit: F) -> Result<(), SecretError>
where
    F: FnOnce(&mut Vault),
{
    let _guard = io_lock();
    let mut vault = load_raw()?;
    edit(&mut vault);
    vault.prune();
    store_raw(&vault)
}

pub fn get_llm_api_key() -> Result<Option<String>, SecretError> {
    Ok(read_vault()?.llm_api_key().map(str::to_string))
}

pub fn set_llm_api_key(value: &str) -> Result<(), SecretError> {
    update_vault(|vault| vault.set_llm_api_key(Some(value.to_string())))
}

pub fn delete_llm_api_key() -> Result<(), SecretError> {
    update_vault(|vault| vault.set_llm_api_key(None))
}

#[cfg(test)]
#[path = "../../unit-tests/config/vault/store.rs"]
mod tests;

#[cfg(test)]
pub use tests::{
    test_clear_llm, test_clear_scope, test_clear_store_fail, test_fail_store, test_vault_json,
};
