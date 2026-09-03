use std::fs;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_entry_id;

pub type ChangedHandler = Arc<dyn Fn() + Send + Sync>;

static WRITE_LOCK: Mutex<()> = Mutex::new(());
static CHANGED_HANDLER: Mutex<Option<ChangedHandler>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub channel: String,
    pub unread: bool,
    pub created_at: String,
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

pub fn produce(channel: &str) -> Result<Record, String> {
    require_channel(channel)?;
    with_write_lock(|| {
        let mut store = load_store_unlocked();
        let record = Record {
            id: random_entry_id(),
            channel: channel.to_string(),
            unread: true,
            created_at: Utc::now().to_rfc3339(),
        };
        store.records.push(record.clone());
        save_store_unlocked(&store)?;
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
