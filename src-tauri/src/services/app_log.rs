//! Process-scoped application execution log.
//!
//! One Spark process = one process session = one JSONL file:
//! `{cache_dir}/app-log/{process_session_id}.jsonl`
//!
//! Not the Assistant diagnostic file. Agent stays in
//! `{cache_dir}/agent-exec/assistant-diagnostic.jsonl`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use chrono::{FixedOffset, Utc};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::config::paths;
use crate::services::id::random_hex12;

pub const SCHEMA_VERSION: u8 = 1;
pub const DIR_NAME: &str = "app-log";
pub const CURRENT_FILE: &str = "current.json";
pub const STDERR_PREFIX: &str = "[app-log]";
pub const BUSINESS_APP: &str = "app";
pub const EVENT_PROCESS_START: &str = "process.start";
pub const EVENT_PROCESS_END: &str = "process.end";

static SESSION: OnceLock<ProcessSession> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(0);
static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Host,
    Ui,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Ui => "ui",
            Self::Native => "native",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "host" => Some(Self::Host),
            "ui" => Some(Self::Ui),
            "native" => Some(Self::Native),
            _ => None,
        }
    }
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ProcessSession {
    id: String,
    started_at: String,
    pid: u32,
}

#[derive(Debug, Serialize)]
struct AppLogLine<'a> {
    schema_version: u8,
    timestamp: String,
    process_session_id: &'a str,
    seq: u64,
    pid: u32,
    side: &'static str,
    level: &'static str,
    business: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<&'a str>,
    event: &'a str,
    params: Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct CurrentPointer<'a> {
    process_session_id: &'a str,
    pid: u32,
    started_at: &'a str,
    path: String,
}

pub fn parse_trace_id(value: &str) -> Option<&str> {
    let value = value.trim();
    if !(8..=64).contains(&value.len())
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(value)
}

pub fn new_trace_id() -> String {
    format!("trace_{}", random_hex12())
}

pub fn token_ok(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

pub fn process_session_id() -> String {
    session().id.clone()
}

pub fn log_dir() -> Result<PathBuf, String> {
    Ok(paths::cache_dir()
        .map_err(|e| format!("{e:?}"))?
        .join(DIR_NAME))
}

pub fn log_path() -> Result<PathBuf, String> {
    Ok(log_dir()?.join(format!("{}.jsonl", session().id)))
}

pub fn current_path() -> Result<PathBuf, String> {
    Ok(log_dir()?.join(CURRENT_FILE))
}

/// Mint the process session, write `current.json`, and emit `process.start`.
pub fn init() {
    let _ = session();
    let _ = write_current_pointer();
    log(
        BUSINESS_APP,
        EVENT_PROCESS_START,
        None,
        Some(
            serde_json::to_value(crate::host::snapshot())
                .expect("package snapshot serializes"),
        ),
        Side::Host,
        Level::Info,
    );
}

/// Emit `process.end` on a graceful Host exit.
pub fn shutdown() {
    if SESSION.get().is_none() {
        return;
    }
    log(
        BUSINESS_APP,
        EVENT_PROCESS_END,
        None,
        None,
        Side::Host,
        Level::Info,
    );
}

pub fn log(
    business: &str,
    event: &str,
    trace_id: Option<&str>,
    params: Option<Value>,
    side: Side,
    level: Level,
) {
    let Some(line) = encode_line(business, event, trace_id, params, side, level) else {
        return;
    };
    eprintln!("{STDERR_PREFIX} {line}");
    let _ = append_line(&line);
}

fn session() -> &'static ProcessSession {
    SESSION.get_or_init(|| ProcessSession {
        id: format!("psess_{}", random_hex12()),
        started_at: timestamp(),
        pid: std::process::id(),
    })
}

fn timestamp() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    Utc::now().with_timezone(&tz).to_rfc3339()
}

fn encode_line(
    business: &str,
    event: &str,
    trace_id: Option<&str>,
    params: Option<Value>,
    side: Side,
    level: Level,
) -> Option<String> {
    if !token_ok(business) || !token_ok(event) {
        return None;
    }
    let session = session();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    serde_json::to_string(&AppLogLine {
        schema_version: SCHEMA_VERSION,
        timestamp: timestamp(),
        process_session_id: &session.id,
        seq,
        pid: session.pid,
        side: side.as_str(),
        level: level.as_str(),
        business,
        trace_id: trace_id.and_then(parse_trace_id),
        event,
        params: sanitize_params(params),
    })
    .ok()
}

fn sanitize_params(params: Option<Value>) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(Value::Object(raw)) = params else {
        return out;
    };
    const RESERVED: &[&str] = &[
        "schema_version",
        "timestamp",
        "process_session_id",
        "seq",
        "pid",
        "side",
        "level",
        "business",
        "event",
        "trace_id",
    ];
    for (key, value) in raw {
        if RESERVED.contains(&key.as_str()) {
            continue;
        }
        out.insert(key, value);
    }
    out
}

fn write_current_pointer() -> Result<(), String> {
    let session = session();
    let dir = log_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = log_path()?;
    let pointer = CurrentPointer {
        process_session_id: &session.id,
        pid: session.pid,
        started_at: &session.started_at,
        path: path.to_string_lossy().into_owned(),
    };
    let body = serde_json::to_string_pretty(&pointer).map_err(|e| e.to_string())?;
    fs::write(current_path()?, body).map_err(|e| e.to_string())
}

fn append_line(line: &str) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    write_line(line)
}

fn write_line(line: &str) -> Result<(), String> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let lock = WRITE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())
}

#[cfg(test)]
pub fn persist_for_test(
    business: &str,
    event: &str,
    trace_id: Option<&str>,
    params: Option<Value>,
    side: Side,
    level: Level,
) -> Result<(), String> {
    let _ = write_current_pointer();
    let line = encode_line(business, event, trace_id, params, side, level)
        .ok_or_else(|| "invalid app log tokens".to_string())?;
    write_line(&line)
}

#[cfg(test)]
#[path = "../unit-tests/services/app_log.rs"]
mod tests;
