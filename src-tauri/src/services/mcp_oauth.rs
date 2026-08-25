//! Host-local MCP OAuth: per-slot tickets, Keychain slot ledger, and config-dir device tickets.

use std::fmt;
use std::sync::Mutex;

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::settings;
use crate::repositories::atomic_json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Workbench,
    CursorIde,
}

impl Slot {
    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Workbench => "workbench",
            Slot::CursorIde => "cursor_ide",
        }
    }

    pub fn parse(name: &str) -> Result<Self, OAuthError> {
        match name {
            "workbench" => Ok(Slot::Workbench),
            "cursor_ide" => Ok(Slot::CursorIde),
            _ => Err(OAuthError::slot_unknown),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TicketHandle(String);

impl TicketHandle {
    pub fn from_secret(secret: impl Into<String>) -> Self {
        Self(secret.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for TicketHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TicketHandle([REDACTED])")
    }
}

impl fmt::Display for TicketHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketState {
    Live,
    Revoked,
}

impl TicketState {
    fn as_str(self) -> &'static str {
        match self {
            TicketState::Live => "live",
            TicketState::Revoked => "revoked",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "live" => Some(TicketState::Live),
            "revoked" => Some(TicketState::Revoked),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerRecord {
    pub slot: Slot,
    pub handle: TicketHandle,
    pub state: TicketState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceRecord {
    pub device_id: String,
    pub device_label: Option<String>,
    pub revoked: bool,
}

const DEVICE_LEDGER_FILE: &str = "device-tickets.json";
static DEVICE_LEDGER_LOCK: Mutex<()> = Mutex::new(());

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthError {
    keychain_unavailable,
    slot_unknown,
    rejected,
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OAuthError::keychain_unavailable => "keychain_unavailable",
            OAuthError::slot_unknown => "slot_unknown",
            OAuthError::rejected => "rejected",
        })
    }
}

impl std::error::Error for OAuthError {}

pub fn issue_for_slot(slot: Slot) -> Result<TicketHandle, OAuthError> {
    if let Some(record) = read_record(slot)? {
        if record.state == TicketState::Live {
            return Ok(record.handle);
        }
    }
    persist_live(slot)
}

pub fn verify_for_slot(slot: Slot, handle: TicketHandle) -> Result<(), OAuthError> {
    match read_record(slot)? {
        Some(record) if record.state == TicketState::Live && record.handle == handle => Ok(()),
        _ => Err(OAuthError::rejected),
    }
}

pub fn revoke_for_slot(slot: Slot) -> Result<(), OAuthError> {
    match read_record(slot)? {
        Some(record) if record.state == TicketState::Live => write_record(&LedgerRecord {
            slot: record.slot,
            handle: record.handle,
            state: TicketState::Revoked,
        }),
        _ => Ok(()),
    }
}

pub fn rotate_for_slot(slot: Slot) -> Result<TicketHandle, OAuthError> {
    if slot != Slot::CursorIde {
        return Err(OAuthError::rejected);
    }
    persist_live(slot)
}

pub fn ledger_record(slot: Slot) -> Result<Option<LedgerRecord>, OAuthError> {
    read_record(slot)
}

pub fn issue_for_device(
    device_id: &str,
    device_label: Option<&str>,
) -> Result<TicketHandle, OAuthError> {
    if device_id.is_empty() {
        return Err(OAuthError::rejected);
    }
    let _lock = DEVICE_LEDGER_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let handle = new_handle();
    let hash = hash_token(handle.as_str());
    let mut ledger = load_device_ledger()?;
    if let Some(row) = ledger
        .devices
        .iter_mut()
        .find(|row| row.device_id == device_id)
    {
        row.device_label = device_label.map(str::to_string);
        row.token_hash = hash;
        row.revoked = false;
    } else {
        ledger.devices.push(DeviceLedgerRow {
            device_id: device_id.to_string(),
            device_label: device_label.map(str::to_string),
            token_hash: hash,
            revoked: false,
        });
    }
    save_device_ledger(&ledger)?;
    Ok(handle)
}

pub fn verify_device_token(token: &str) -> Result<String, OAuthError> {
    let _lock = DEVICE_LEDGER_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let hash = hash_token(token);
    let ledger = load_device_ledger()?;
    ledger
        .devices
        .into_iter()
        .find(|row| row.token_hash == hash && !row.revoked)
        .map(|row| row.device_id)
        .ok_or(OAuthError::rejected)
}

pub fn revoke_for_device(device_id: &str) -> Result<(), OAuthError> {
    let _lock = DEVICE_LEDGER_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut ledger = load_device_ledger()?;
    if let Some(row) = ledger
        .devices
        .iter_mut()
        .find(|row| row.device_id == device_id)
    {
        row.revoked = true;
        save_device_ledger(&ledger)?;
    }
    Ok(())
}

pub fn list_devices() -> Result<Vec<DeviceRecord>, OAuthError> {
    let _lock = DEVICE_LEDGER_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let ledger = load_device_ledger()?;
    Ok(ledger
        .devices
        .into_iter()
        .map(|row| DeviceRecord {
            device_id: row.device_id,
            device_label: row.device_label,
            revoked: row.revoked,
        })
        .collect())
}

#[cfg_attr(test, allow(dead_code))]
fn keyring_service() -> &'static str {
    #[cfg(test)]
    {
        "lulu-workbench-mcp-oauth-test"
    }
    #[cfg(not(test))]
    {
        "lulu-workbench-mcp-oauth"
    }
}

#[cfg(test)]
static FORCE_KEYCHAIN_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_SLOT_LEDGER: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

#[cfg(test)]
pub fn test_force_keychain_unavailable(on: bool) {
    FORCE_KEYCHAIN_UNAVAILABLE.store(on, Ordering::SeqCst);
}

fn ensure_keychain_available() -> Result<(), OAuthError> {
    #[cfg(test)]
    if FORCE_KEYCHAIN_UNAVAILABLE.load(Ordering::SeqCst) {
        return Err(OAuthError::keychain_unavailable);
    }
    Ok(())
}

#[cfg(test)]
fn test_slot_ledger() -> std::sync::MutexGuard<'static, Option<HashMap<String, String>>> {
    TEST_SLOT_LEDGER.lock().unwrap_or_else(|e| e.into_inner())
}

fn read_record(slot: Slot) -> Result<Option<LedgerRecord>, OAuthError> {
    ensure_keychain_available()?;
    #[cfg(test)]
    {
        let mut guard = test_slot_ledger();
        let store = guard.get_or_insert_with(HashMap::new);
        return match store.get(slot.as_str()) {
            Some(raw) => parse_record(slot, raw).map(Some),
            None => Ok(None),
        };
    }
    #[cfg(not(test))]
    {
        let entry = keyring::Entry::new(keyring_service(), slot.as_str())
            .map_err(|_| OAuthError::keychain_unavailable)?;
        match entry.get_password() {
            Ok(raw) => parse_record(slot, &raw).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(OAuthError::keychain_unavailable),
        }
    }
}

fn persist_live(slot: Slot) -> Result<TicketHandle, OAuthError> {
    let handle = new_handle();
    write_record(&LedgerRecord {
        slot,
        handle: handle.clone(),
        state: TicketState::Live,
    })?;
    Ok(handle)
}

fn write_record(record: &LedgerRecord) -> Result<(), OAuthError> {
    ensure_keychain_available()?;
    #[cfg(test)]
    {
        let mut guard = test_slot_ledger();
        let store = guard.get_or_insert_with(HashMap::new);
        store.insert(record.slot.as_str().to_string(), serialize_record(record));
        return Ok(());
    }
    #[cfg(not(test))]
    {
        let entry = keyring::Entry::new(keyring_service(), record.slot.as_str())
            .map_err(|_| OAuthError::keychain_unavailable)?;
        entry
            .set_password(&serialize_record(record))
            .map_err(|_| OAuthError::keychain_unavailable)
    }
}

fn serialize_record(record: &LedgerRecord) -> String {
    serde_json::json!({
        "handle": record.handle.as_str(),
        "state": record.state.as_str(),
    })
    .to_string()
}

fn parse_record(slot: Slot, raw: &str) -> Result<LedgerRecord, OAuthError> {
    let value: Value = serde_json::from_str(raw).map_err(|_| OAuthError::keychain_unavailable)?;
    let handle = value
        .get("handle")
        .and_then(Value::as_str)
        .ok_or(OAuthError::keychain_unavailable)?;
    let state = value
        .get("state")
        .and_then(Value::as_str)
        .and_then(TicketState::parse)
        .ok_or(OAuthError::keychain_unavailable)?;
    Ok(LedgerRecord {
        slot,
        handle: TicketHandle::from_secret(handle),
        state,
    })
}

fn new_handle() -> TicketHandle {
    let mut bytes = [0u8; 32];
    if !read_random_bytes(&mut bytes) {
        fill_weak_random(&mut bytes);
    }
    TicketHandle(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn read_random_bytes(buf: &mut [u8]) -> bool {
    use std::io::Read;
    #[cfg(unix)]
    {
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            return file.read_exact(buf).is_ok();
        }
    }
    let _ = buf;
    false
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct DeviceLedger {
    #[serde(default)]
    devices: Vec<DeviceLedgerRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeviceLedgerRow {
    device_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device_label: Option<String>,
    token_hash: String,
    revoked: bool,
}

fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn device_ledger_path() -> Result<std::path::PathBuf, OAuthError> {
    Ok(settings::settings_config_dir()
        .map_err(|_| OAuthError::rejected)?
        .join(DEVICE_LEDGER_FILE))
}

fn load_device_ledger() -> Result<DeviceLedger, OAuthError> {
    let path = device_ledger_path()?;
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|_| OAuthError::rejected),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(DeviceLedger::default()),
        Err(_) => Err(OAuthError::rejected),
    }
}

fn save_device_ledger(ledger: &DeviceLedger) -> Result<(), OAuthError> {
    let path = device_ledger_path()?;
    let value = serde_json::to_value(ledger).map_err(|_| OAuthError::rejected)?;
    atomic_json::write_json(&path, &value).map_err(|_| OAuthError::rejected)
}

fn fill_weak_random(buf: &mut [u8]) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    for (index, slot) in buf.iter_mut().enumerate() {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        index.hash(&mut hasher);
        *slot = (hasher.finish() & 0xFF) as u8;
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_oauth_tests.rs"]
mod tests;
