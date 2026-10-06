//! Host-local MCP OAuth: per-slot tickets and the bind device ledger, both in the vault.

use std::fmt;
use std::sync::Mutex;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::config::vault::{self, BindDevice, SlotTicket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Spark,
    CursorIde,
}

impl Slot {
    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Spark => "spark",
            Slot::CursorIde => "cursor_ide",
        }
    }

    pub fn parse(name: &str) -> Result<Self, OAuthError> {
        match name {
            "spark" => Ok(Slot::Spark),
            "cursor" | "cursor_ide" | "codex" | "claude" => Ok(Slot::CursorIde),
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
    pub(crate) fn as_str(self) -> &'static str {
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
    pub token_hint: Option<String>,
}

static DEVICE_LEDGER_LOCK: Mutex<()> = Mutex::new(());

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthError {
    store_unavailable,
    slot_unknown,
    rejected,
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OAuthError::store_unavailable => "store_unavailable",
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
    let hint = token_hint(handle.as_str());
    let mut ledger = load_device_ledger()?;
    if let Some(row) = ledger
        .devices
        .iter_mut()
        .find(|row| row.device_id == device_id)
    {
        row.device_label = device_label.map(str::to_string);
        row.token_hash = hash;
        row.token_hint = Some(hint);
        row.revoked = false;
    } else {
        ledger.devices.push(BindDevice {
            device_id: device_id.to_string(),
            device_label: device_label.map(str::to_string),
            token_hash: hash,
            token_hint: Some(hint),
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
            token_hint: row.token_hint,
        })
        .collect())
}

pub fn token_hint(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.len() < 4 {
        return "••••".to_string();
    }
    format!("••••{}", &trimmed[trimmed.len() - 4..])
}

pub fn ticket_view(channel: &str) -> Result<Value, OAuthError> {
    match channel {
        "spark" => slot_ticket_view(Slot::Spark, false),
        "cursor" | "cursor_ide" | "codex" | "claude" => {
            let mut value = slot_ticket_view(Slot::CursorIde, true)?;
            value["channel"] = json!(channel);
            Ok(value)
        }
        "mobile" => mobile_ticket_view(),
        _ => Err(OAuthError::slot_unknown),
    }
}

fn slot_ticket_view(slot: Slot, include_handle: bool) -> Result<Value, OAuthError> {
    match ledger_record(slot)? {
        None => Ok(json!({
            "channel": slot.as_str(),
            "state": "none",
        })),
        Some(record) => {
            let mut value = json!({
                "channel": slot.as_str(),
                "state": record.state.as_str(),
                "hint": token_hint(record.handle.as_str()),
            });
            if include_handle && record.state == TicketState::Live {
                value["handle"] = json!(record.handle.as_str());
            }
            Ok(value)
        }
    }
}

fn mobile_ticket_view() -> Result<Value, OAuthError> {
    let devices = list_devices()?;
    Ok(json!({
        "channel": "mobile",
        "devices": devices.iter().map(|row| json!({
            "device_id": row.device_id,
            "device_label": row.device_label,
            "revoked": row.revoked,
            "hint": row.token_hint,
        })).collect::<Vec<_>>(),
    }))
}

fn map_vault<T>(result: Result<T, vault::SecretError>) -> Result<T, OAuthError> {
    result.map_err(|_| OAuthError::store_unavailable)
}

fn ticket_from_record(record: &LedgerRecord) -> SlotTicket {
    SlotTicket {
        handle: record.handle.as_str().to_string(),
        state: record.state.as_str().to_string(),
    }
}

fn record_from_ticket(slot: Slot, ticket: &SlotTicket) -> Result<LedgerRecord, OAuthError> {
    let state = TicketState::parse(&ticket.state).ok_or(OAuthError::store_unavailable)?;
    Ok(LedgerRecord {
        slot,
        handle: TicketHandle::from_secret(ticket.handle.clone()),
        state,
    })
}

fn read_record(slot: Slot) -> Result<Option<LedgerRecord>, OAuthError> {
    let vault = map_vault(vault::read_vault())?;
    match vault.mcp_slot(slot.as_str()) {
        Some(ticket) => record_from_ticket(slot, ticket).map(Some),
        None => Ok(None),
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
    map_vault(vault::update_vault(|doc| {
        doc.set_mcp_slot(record.slot.as_str(), Some(ticket_from_record(record)));
    }))
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

#[derive(Clone, Debug, Default)]
struct DeviceLedger {
    devices: Vec<BindDevice>,
}

fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn load_device_ledger() -> Result<DeviceLedger, OAuthError> {
    let vault = map_vault(vault::read_vault())?;
    Ok(DeviceLedger {
        devices: vault.devices().to_vec(),
    })
}

fn save_device_ledger(ledger: &DeviceLedger) -> Result<(), OAuthError> {
    map_vault(vault::update_vault(|doc| {
        doc.set_devices(ledger.devices.clone());
    }))
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
#[path = "../unit-tests/services/mcp_oauth.rs"]
mod tests;
