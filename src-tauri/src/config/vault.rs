//! One secret tree for LLM, MCP slot tickets, and the bind device ledger.
//!
//! Release: Keychain `lulu-spark` / `vault` (JSON). Debug: the same tree in
//! `dev-secrets.toml`. Tests and `TestSandbox` stay in process memory.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::config::settings;

pub const KEYCHAIN_SERVICE: &str = "lulu-spark";
pub const ACCOUNT_VAULT: &str = "vault";
pub const KEY_LLM_API_KEY: &str = "llm_api_key";

const SLOT_SPARK: &str = "spark";
const SLOT_CURSOR_IDE: &str = "cursor_ide";

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

impl std::error::Error for SecretError {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmSecrets>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "mcp-oauth")]
    pub mcp_oauth: Option<McpOauthSecrets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<BindSecrets>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmSecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_api_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOauthSecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spark: Option<SlotTicket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_ide: Option<SlotTicket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotTicket {
    pub handle: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindSecrets {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<BindDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindDevice {
    pub device_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_label: Option<String>,
    pub token_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_hint: Option<String>,
    pub revoked: bool,
}

impl Vault {
    pub fn is_empty(&self) -> bool {
        self.llm_api_key().is_none()
            && self.mcp_slot(SLOT_SPARK).is_none()
            && self.mcp_slot(SLOT_CURSOR_IDE).is_none()
            && self.devices().is_empty()
    }

    pub fn llm_api_key(&self) -> Option<&str> {
        self.llm
            .as_ref()
            .and_then(|llm| llm.llm_api_key.as_deref())
            .filter(|key| !key.is_empty())
    }

    pub fn set_llm_api_key(&mut self, value: Option<String>) {
        match value.filter(|key| !key.is_empty()) {
            Some(key) => {
                self.llm = Some(LlmSecrets {
                    llm_api_key: Some(key),
                });
            }
            None => self.llm = None,
        }
    }

    pub fn mcp_slot(&self, slot: &str) -> Option<&SlotTicket> {
        let mcp = self.mcp_oauth.as_ref()?;
        match slot {
            SLOT_SPARK => mcp.spark.as_ref(),
            SLOT_CURSOR_IDE => mcp.cursor_ide.as_ref(),
            _ => None,
        }
    }

    pub fn set_mcp_slot(&mut self, slot: &str, ticket: Option<SlotTicket>) {
        let mcp = self.mcp_oauth.get_or_insert_with(McpOauthSecrets::default);
        match slot {
            SLOT_SPARK => mcp.spark = ticket,
            SLOT_CURSOR_IDE => mcp.cursor_ide = ticket,
            _ => {}
        }
        if mcp.spark.is_none() && mcp.cursor_ide.is_none() {
            self.mcp_oauth = None;
        }
    }

    pub fn devices(&self) -> &[BindDevice] {
        self.bind
            .as_ref()
            .map(|bind| bind.devices.as_slice())
            .unwrap_or(&[])
    }

    pub fn set_devices(&mut self, devices: Vec<BindDevice>) {
        if devices.is_empty() {
            self.bind = None;
        } else {
            self.bind = Some(BindSecrets { devices });
        }
    }

    fn prune(&mut self) {
        if self.llm_api_key().is_none() {
            self.llm = None;
        }
        if let Some(mcp) = &self.mcp_oauth {
            if mcp.spark.is_none() && mcp.cursor_ide.is_none() {
                self.mcp_oauth = None;
            }
        }
        if self.devices().is_empty() {
            self.bind = None;
        }
    }
}

pub fn vault_to_json(vault: &Vault) -> Result<String, SecretError> {
    serde_json::to_string(vault).map_err(|e| SecretError::Keyring(e.to_string()))
}

pub fn vault_from_json(raw: &str) -> Result<Vault, SecretError> {
    if raw.trim().is_empty() {
        return Ok(Vault::default());
    }
    serde_json::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))
}

pub fn vault_to_toml(vault: &Vault) -> Result<String, SecretError> {
    if vault.is_empty() {
        return Ok(String::new());
    }
    toml::to_string(vault).map_err(|e| SecretError::Keyring(e.to_string()))
}

pub fn vault_from_toml(raw: &str) -> Result<Vault, SecretError> {
    if raw.trim().is_empty() {
        return Ok(Vault::default());
    }
    toml::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))
}

fn io_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn scope_id() -> String {
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

fn memory_vault() -> Vault {
    memory_map()
        .get(&scope_id())
        .cloned()
        .unwrap_or_default()
}

fn store_memory_vault(vault: &Vault) {
    memory_map().insert(scope_id(), vault.clone());
}

fn load_raw() -> Result<Vault, SecretError> {
    if settings::uses_in_memory_keychain() {
        Ok(memory_vault())
    } else {
        persistent_load()
    }
}

fn store_raw(vault: &Vault) -> Result<(), SecretError> {
    if settings::uses_in_memory_keychain() {
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
        std::fs::create_dir_all(parent).map_err(|e| SecretError::Keyring(e.to_string()))?;
    }
    let content = vault_to_toml(vault)?;
    std::fs::write(path, content).map_err(|e| SecretError::Keyring(e.to_string()))
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
        .map_err(|e| SecretError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(SecretError::Keyring(e.to_string())),
    }
}

#[cfg(not(debug_assertions))]
fn keychain_set(service: &str, account: &str, value: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new(service, account)
        .map_err(|e| SecretError::Keyring(e.to_string()))?;
    entry
        .set_password(value)
        .map_err(|e| SecretError::Keyring(e.to_string()))
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
#[path = "../unit-tests/config/vault.rs"]
mod tests;

#[cfg(test)]
pub use tests::{test_clear_llm, test_clear_scope, test_vault_json};
