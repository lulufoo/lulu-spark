use std::fs;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::app_log::{self, Level, Side};
use crate::services::id::random_entry_id;
use crate::services::os_notify_trace::{
    new_trace_id, parse_trace_id, NODE_MESSAGE_CENTER_RECEIVED,
};

pub type ChangedHandler = Arc<dyn Fn() + Send + Sync>;

static WRITE_LOCK: Mutex<()> = Mutex::new(());
static CHANGED_HANDLER: Mutex<Option<ChangedHandler>> = Mutex::new(None);
static LAST_CHANGED_ENVELOPE: Mutex<Option<Envelope>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub channel: String,
    pub unread: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub business: String,
    pub action: String,
    pub params: serde_json::Value,
}

impl From<&str> for Envelope {
    fn from(channel: &str) -> Self {
        Self {
            business: channel.to_string(),
            action: "notify".to_string(),
            params: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Store {
    version: u32,
    records: Vec<Record>,
}

fn default_store() -> Store {
    Store {
        version: 1,
        records: vec![],
    }
}

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

fn allowed_channel(channel: &str) -> bool {
    matches!(channel, "notes" | "read_later" | "todos")
}

fn require_channel(channel: &str) -> Result<(), String> {
    if allowed_channel(channel) {
        Ok(())
    } else {
        Err(format!("unsupported channel: {channel}"))
    }
}

fn require_envelope(envelope: &Envelope) -> Result<(), String> {
    require_channel(&envelope.business)?;
    if envelope.action.is_empty() {
        return Err("action must not be empty".to_string());
    }
    if !envelope.params.is_object() {
        return Err("params must be a JSON object".to_string());
    }
    Ok(())
}

pub fn set_changed_handler(handler: Option<ChangedHandler>) {
    *CHANGED_HANDLER
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = handler;
}

fn notify_changed() {
    let handler = CHANGED_HANDLER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(handler) = handler {
        handler();
    }
}

pub fn last_changed_envelope() -> Option<Envelope> {
    LAST_CHANGED_ENVELOPE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn store_last_changed_envelope(envelope: Envelope) {
    *LAST_CHANGED_ENVELOPE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(envelope);
}

fn load_store_unlocked() -> Store {
    let path = match paths::message_center_path() {
        Ok(p) => p,
        Err(_) => return default_store(),
    };
    if !path.is_file() {
        return default_store();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return default_store();
    };
    serde_json::from_str(&text).unwrap_or_else(|_| default_store())
}

fn save_store_unlocked(store: &Store) -> Result<(), String> {
    let path = paths::message_center_path().map_err(|e| format!("{e:?}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let value = serde_json::to_value(store).map_err(|e| e.to_string())?;
    atomic_json::write_json(&path, &value)
}

fn ensure_hop_id(params: &mut serde_json::Value) {
    let Some(map) = params.as_object_mut() else {
        return;
    };
    let keep = map
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(parse_trace_id)
        .map(str::to_string);
    if keep.is_none() {
        map.insert("id".to_string(), serde_json::json!(new_trace_id()));
    }
}

pub fn produce(envelope: impl Into<Envelope>) -> Result<Record, String> {
    let mut envelope = envelope.into();
    require_envelope(&envelope)?;
    ensure_hop_id(&mut envelope.params);
    with_write_lock(|| {
        let mut store = load_store_unlocked();
        let record = Record {
            id: random_entry_id(),
            channel: envelope.business.clone(),
            unread: true,
            created_at: Utc::now().to_rfc3339(),
        };
        store.records.push(record.clone());
        save_store_unlocked(&store)?;
        let hop_id = envelope
            .params
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("trace_missing");
        app_log::log(
            &envelope.business,
            NODE_MESSAGE_CENTER_RECEIVED,
            Some(hop_id),
            Some(serde_json::json!({ "outcome": "ok" })),
            Side::Host,
            Level::Info,
        );
        store_last_changed_envelope(envelope);
        notify_changed();
        Ok(record)
    })
}

pub fn channel_unread(channel: &str) -> bool {
    if !allowed_channel(channel) {
        return false;
    }
    with_write_lock(|| {
        load_store_unlocked()
            .records
            .iter()
            .any(|record| record.channel == channel && record.unread)
    })
}

pub fn mark_channel_read(channel: &str) -> Result<(), String> {
    require_channel(channel)?;
    with_write_lock(|| {
        let mut store = load_store_unlocked();
        for record in &mut store.records {
            if record.channel == channel {
                record.unread = false;
            }
        }
        save_store_unlocked(&store)
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/message_center.rs"]
mod tests;
