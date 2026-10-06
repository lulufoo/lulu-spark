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

const LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH: &str = "lulu-spark-mcp-oauth";
const ACCOUNT_LLM_LEGACY: &str = "llm_api_key";
const SLOT_SPARK: &str = "spark";
const SLOT_CURSOR_IDE: &str = "cursor_ide";
const DEVICE_LEDGER_FILE: &str = "device-tickets.json";

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DeviceLedgerFile {
    #[serde(default)]
    devices: Vec<BindDevice>,
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
    Ok(parse_dev_secrets(raw)?.vault)
}

struct DevSecretsFile {
    vault: Vault,
    leftover_llm: Option<String>,
}

fn parse_dev_secrets(raw: &str) -> Result<DevSecretsFile, SecretError> {
    if raw.trim().is_empty() {
        return Ok(DevSecretsFile {
            vault: Vault::default(),
            leftover_llm: None,
        });
    }
    let value: toml::Value =
        toml::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))?;
    let table = match value {
        toml::Value::Table(table) => table,
        _ => {
            return Ok(DevSecretsFile {
                vault: Vault::default(),
                leftover_llm: None,
            })
        }
    };
    let leftover_llm = table
        .get(ACCOUNT_LLM_LEGACY)
        .and_then(toml::Value::as_str)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    let mut vault_table = toml::map::Map::new();
    for key in ["llm", "mcp-oauth", "bind"] {
        if let Some(nested) = table.get(key) {
            vault_table.insert(key.to_string(), nested.clone());
        }
    }
    let vault: Vault = toml::Value::Table(vault_table)
        .try_into()
        .map_err(|e| SecretError::Keyring(e.to_string()))?;
    Ok(DevSecretsFile { vault, leftover_llm })
}

fn io_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Default)]
struct LegacyMemory {
    llm_api_key: Option<String>,
    slots: HashMap<String, String>,
    devices: Option<Vec<BindDevice>>,
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

fn legacy_map() -> std::sync::MutexGuard<'static, HashMap<String, LegacyMemory>> {
    static LEGACY: OnceLock<Mutex<HashMap<String, LegacyMemory>>> = OnceLock::new();
    LEGACY
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

fn legacy_row<R>(edit: impl FnOnce(&mut LegacyMemory) -> R) -> R {
    let mut map = legacy_map();
    edit(map.entry(scope_id()).or_default())
}

fn take_legacy_row() -> LegacyMemory {
    legacy_map().remove(&scope_id()).unwrap_or_default()
}

fn peek_legacy_row() -> LegacyMemory {
    legacy_map()
        .get(&scope_id())
        .cloned()
        .unwrap_or_default()
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
    Ok(parse_dev_secrets(&raw)?.vault)
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

#[cfg(debug_assertions)]
fn keychain_get_or_skip(service: &str, account: &str) -> Result<Option<String>, SecretError> {
    let entry = match keyring::Entry::new(service, account) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
    };
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

fn keychain_delete(service: &str, account: &str) -> Result<(), SecretError> {
    let entry = match keyring::Entry::new(service, account) {
        Ok(entry) => entry,
        Err(_) => return Ok(()),
    };
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(SecretError::Keyring(e.to_string())),
    }
}

fn device_ledger_path() -> Option<std::path::PathBuf> {
    settings::settings_config_dir()
        .ok()
        .map(|dir| dir.join(DEVICE_LEDGER_FILE))
}

fn parse_device_ledger_json(raw: &str) -> Result<Vec<BindDevice>, SecretError> {
    let file: DeviceLedgerFile =
        serde_json::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))?;
    Ok(file.devices)
}

fn read_device_ledger_file() -> Result<Option<Vec<BindDevice>>, SecretError> {
    let Some(path) = device_ledger_path() else {
        return Ok(None);
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => Ok(Some(parse_device_ledger_json(&raw)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(SecretError::Keyring(err.to_string())),
    }
}

fn delete_device_ledger_file() -> Result<(), SecretError> {
    let Some(path) = device_ledger_path() else {
        return Ok(());
    };
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(SecretError::Keyring(err.to_string())),
    }
}

fn parse_slot_ticket(raw: &str) -> Result<SlotTicket, SecretError> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))?;
    let handle = value
        .get("handle")
        .and_then(serde_json::Value::as_str)
        .filter(|handle| !handle.is_empty())
        .ok_or_else(|| SecretError::Keyring("legacy slot missing handle".into()))?;
    let state = value
        .get("state")
        .and_then(serde_json::Value::as_str)
        .filter(|state| *state == "live" || *state == "revoked")
        .ok_or_else(|| SecretError::Keyring("legacy slot missing state".into()))?;
    Ok(SlotTicket {
        handle: handle.to_string(),
        state: state.to_string(),
    })
}

struct LegacySnapshot {
    llm: Option<String>,
    spark: Option<SlotTicket>,
    cursor_ide: Option<SlotTicket>,
    devices: Option<Vec<BindDevice>>,
}

fn collect_legacy() -> Result<LegacySnapshot, SecretError> {
    if settings::uses_in_memory_keychain() {
        return collect_legacy_memory();
    }
    collect_legacy_persistent()
}

fn collect_legacy_memory() -> Result<LegacySnapshot, SecretError> {
    let row = peek_legacy_row();
    let mut snap = LegacySnapshot {
        llm: row.llm_api_key,
        spark: None,
        cursor_ide: None,
        devices: row.devices,
    };
    if let Some(raw) = row.slots.get(SLOT_SPARK) {
        snap.spark = Some(parse_slot_ticket(raw)?);
    }
    if let Some(raw) = row.slots.get(SLOT_CURSOR_IDE) {
        snap.cursor_ide = Some(parse_slot_ticket(raw)?);
    }
    Ok(snap)
}

fn collect_legacy_persistent() -> Result<LegacySnapshot, SecretError> {
    let mut snap = LegacySnapshot {
        llm: None,
        spark: None,
        cursor_ide: None,
        devices: read_device_ledger_file()?,
    };

    #[cfg(debug_assertions)]
    {
        let path = settings::prod_config_dir().join("dev-secrets.toml");
        if let Ok(raw) = std::fs::read_to_string(path) {
            snap.llm = parse_dev_secrets(&raw)?.leftover_llm;
        }
        if snap.llm.is_none() {
            if let Some(legacy) = keychain_get_or_skip(KEYCHAIN_SERVICE, ACCOUNT_LLM_LEGACY)? {
                snap.llm = Some(legacy);
            }
        }
        if let Some(raw) = keychain_get_or_skip(LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH, SLOT_SPARK)? {
            snap.spark = Some(parse_slot_ticket(&raw)?);
        }
        if let Some(raw) =
            keychain_get_or_skip(LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH, SLOT_CURSOR_IDE)?
        {
            snap.cursor_ide = Some(parse_slot_ticket(&raw)?);
        }
    }

    #[cfg(not(debug_assertions))]
    {
        snap.llm = keychain_get(KEYCHAIN_SERVICE, ACCOUNT_LLM_LEGACY)?;
        if let Some(raw) = keychain_get(LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH, SLOT_SPARK)? {
            snap.spark = Some(parse_slot_ticket(&raw)?);
        }
        if let Some(raw) = keychain_get(LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH, SLOT_CURSOR_IDE)? {
            snap.cursor_ide = Some(parse_slot_ticket(&raw)?);
        }
    }

    Ok(snap)
}

fn merge_legacy(vault: &mut Vault, legacy: &LegacySnapshot) -> bool {
    let mut changed = false;
    if vault.llm_api_key().is_none() {
        if let Some(key) = legacy.llm.clone().filter(|key| !key.is_empty()) {
            vault.set_llm_api_key(Some(key));
            changed = true;
        }
    }
    if vault.mcp_slot(SLOT_SPARK).is_none() {
        if let Some(ticket) = legacy.spark.clone() {
            vault.set_mcp_slot(SLOT_SPARK, Some(ticket));
            changed = true;
        }
    }
    if vault.mcp_slot(SLOT_CURSOR_IDE).is_none() {
        if let Some(ticket) = legacy.cursor_ide.clone() {
            vault.set_mcp_slot(SLOT_CURSOR_IDE, Some(ticket));
            changed = true;
        }
    }
    if vault.devices().is_empty() {
        if let Some(devices) = legacy.devices.clone().filter(|rows| !rows.is_empty()) {
            vault.set_devices(devices);
            changed = true;
        }
    }
    vault.prune();
    changed
}

fn confirm_persisted(expected: &Vault) -> Result<(), SecretError> {
    let read_back = load_raw()?;
    if read_back != *expected {
        return Err(SecretError::Keyring(
            "vault migrate read-back mismatch".into(),
        ));
    }
    Ok(())
}

fn delete_legacy_sources() -> Result<(), SecretError> {
    if settings::uses_in_memory_keychain() {
        let _ = take_legacy_row();
        return Ok(());
    }

    let _ = keychain_delete(KEYCHAIN_SERVICE, ACCOUNT_LLM_LEGACY);
    let _ = keychain_delete(LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH, SLOT_SPARK);
    let _ = keychain_delete(LEGACY_KEYCHAIN_SERVICE_MCP_OAUTH, SLOT_CURSOR_IDE);
    delete_device_ledger_file()?;
    Ok(())
}

fn migrate_locked() -> Result<(), SecretError> {
    let mut vault = load_raw()?;
    let legacy = collect_legacy()?;
    if merge_legacy(&mut vault, &legacy) {
        store_raw(&vault)?;
        confirm_persisted(&vault)?;
        delete_legacy_sources()?;
    } else if !vault.devices().is_empty() && !settings::uses_in_memory_keychain() {
        delete_device_ledger_file()?;
    }
    Ok(())
}

pub fn migrate_legacy_secrets() -> Result<(), SecretError> {
    let _guard = io_lock();
    migrate_locked()
}

pub fn read_vault() -> Result<Vault, SecretError> {
    let _guard = io_lock();
    migrate_locked()?;
    load_raw()
}

pub fn update_vault<F>(edit: F) -> Result<(), SecretError>
where
    F: FnOnce(&mut Vault),
{
    let _guard = io_lock();
    migrate_locked()?;
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
