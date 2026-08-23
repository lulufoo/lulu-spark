//! Host-local MCP OAuth: per-slot tickets and Keychain ledger.

use std::fmt;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

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
    let handle = new_handle();
    write_record(&LedgerRecord {
        slot,
        handle: handle.clone(),
        state: TicketState::Live,
    })?;
    Ok(handle)
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
    let handle = new_handle();
    write_record(&LedgerRecord {
        slot,
        handle: handle.clone(),
        state: TicketState::Live,
    })?;
    Ok(handle)
}

pub fn ledger_record(slot: Slot) -> Result<Option<LedgerRecord>, OAuthError> {
    read_record(slot)
}

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
fn test_force_keychain_unavailable(on: bool) {
    FORCE_KEYCHAIN_UNAVAILABLE.store(on, Ordering::SeqCst);
}

fn ensure_keychain_available() -> Result<(), OAuthError> {
    #[cfg(test)]
    if FORCE_KEYCHAIN_UNAVAILABLE.load(Ordering::SeqCst) {
        return Err(OAuthError::keychain_unavailable);
    }
    Ok(())
}

fn keychain_entry(slot: Slot) -> Result<keyring::Entry, OAuthError> {
    ensure_keychain_available()?;
    keyring::Entry::new(keyring_service(), slot.as_str())
        .map_err(|_| OAuthError::keychain_unavailable)
}

fn read_record(slot: Slot) -> Result<Option<LedgerRecord>, OAuthError> {
    let entry = keychain_entry(slot)?;
    match entry.get_password() {
        Ok(raw) => parse_record(slot, &raw).map(Some),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(OAuthError::keychain_unavailable),
    }
}

fn write_record(record: &LedgerRecord) -> Result<(), OAuthError> {
    let entry = keychain_entry(record.slot)?;
    entry
        .set_password(&serialize_record(record))
        .map_err(|_| OAuthError::keychain_unavailable)
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
