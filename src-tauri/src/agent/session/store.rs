use std::fs;
use std::path::PathBuf;

use chrono::{Local, TimeZone};
use serde_json::{json, Value};

use crate::config::paths;
use crate::services::id::random_entry_id;

use super::types::{Session, Turn};

pub fn agent_dir() -> Result<PathBuf, String> {
    Ok(paths::cache_dir()
        .map_err(|e| format!("{e:?}"))?
        .join("agent"))
}

pub fn agent_log_dir() -> Result<PathBuf, String> {
    agent_dir()
}

/// Conversation files live under `{cache_dir}/agent/sessions`.
/// Tests must use `TestSandbox` so they cannot read or write the user's store.
pub fn sessions_dir() -> Result<PathBuf, String> {
    let dir = agent_dir()?.join("sessions");
    #[cfg(test)]
    {
        reject_unisolated_sessions_dir(&dir)?;
    }
    Ok(dir)
}

#[cfg(test)]
fn reject_unisolated_sessions_dir(dir: &PathBuf) -> Result<(), String> {
    use crate::config::settings;
    if !settings::is_test_sandbox() {
        return Err("assistant conversation directory requires TestSandbox isolation".into());
    }
    let prod_cache = settings::load_prod_settings().cache_dir;
    if dir == &prod_cache || dir.starts_with(&prod_cache) {
        return Err(format!(
            "assistant conversation directory must not use prod cache {}",
            prod_cache.display()
        ));
    }
    Ok(())
}

pub fn session_file_path(session_id: &str) -> Result<PathBuf, String> {
    let id = session_id.trim();
    if id.is_empty() {
        return Err("Missing session_id".into());
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("Invalid session_id".into());
    }
    Ok(sessions_dir()?.join(format!("{id}.json")))
}

pub fn create_session() -> Result<Session, String> {
    let session = Session {
        session_id: format!("workbench_chat_{}", random_entry_id()),
        turns: Vec::new(),
        staged: Vec::new(),
    };
    save_session(&session)?;
    Ok(session)
}

pub fn save_session(session: &Session) -> Result<(), String> {
    let path = session_file_path(&session.session_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_session(session_id: &str) -> Result<Session, String> {
    let path = session_file_path(session_id)?;
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn delete_session(session_id: &str) -> Result<(), String> {
    let path = session_file_path(session_id)?;
    if !path.is_file() {
        return Err("Session not found".into());
    }
    fs::remove_file(&path).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    if tmp.is_file() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(())
}

const HOME_CHAT_LIST_LIMIT: usize = 20;

fn format_session_when(updated_at: i64) -> String {
    if updated_at <= 0 {
        return "New conversation".to_string();
    }
    Local
        .timestamp_opt(updated_at, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "New conversation".to_string())
}

fn session_list_title(session: &Session, updated_at: i64) -> String {
    session
        .turns
        .iter()
        .find(|turn| turn.role == "user")
        .and_then(|turn| turn.content.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|text| {
            let clipped: String = text.chars().take(48).collect();
            if text.chars().count() > 48 {
                format!("{clipped}…")
            } else {
                clipped
            }
        })
        .unwrap_or_else(|| format_session_when(updated_at))
}

fn file_updated_at(path: &PathBuf) -> i64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Disk session catalog for the Home history list. Newest first.
pub fn list_session_summaries() -> Result<Vec<Value>, String> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut items: Vec<(i64, Value)> = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem.ends_with(".json") || stem.contains('.') {
            continue;
        }
        let Ok(session) = load_session(stem) else {
            continue;
        };
        let updated_at = file_updated_at(&path);
        items.push((
            updated_at,
            json!({
                "session_id": session.session_id,
                "title": session_list_title(&session, updated_at),
                "updated_at": updated_at,
            }),
        ));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items.truncate(HOME_CHAT_LIST_LIMIT);
    Ok(items.into_iter().map(|(_, v)| v).collect())
}

/// Read-only turns for Home / binding hydrate. Empty when no live session / load fails.
///
/// User and assistant text only. Tool dumps stay on disk and must not ride the
/// IPC — a single `list_todo_tasks` result can be hundreds of KB.
pub fn load_turns_value(session_id: &str) -> Value {
    let id = session_id.trim();
    if id.is_empty() {
        return Value::Array(Vec::new());
    }
    match load_session(id) {
        Ok(session) => Value::Array(
            session
                .turns
                .iter()
                .filter(|turn| {
                    (turn.role == "user" || turn.role == "assistant")
                        && turn
                            .content
                            .as_deref()
                            .map(|text| !text.is_empty())
                            .unwrap_or(false)
                })
                .map(|turn| {
                    json!({
                        "role": turn.role,
                        "content": turn.content,
                    })
                })
                .collect(),
        ),
        Err(_) => Value::Array(Vec::new()),
    }
}

/// Read-only staged registrations for Home / binding hydrate. Empty when no live session / load fails.
pub fn load_staged_value(session_id: &str) -> Value {
    let id = session_id.trim();
    if id.is_empty() {
        return Value::Array(Vec::new());
    }
    match load_session(id) {
        Ok(session) => serde_json::to_value(&session.staged).unwrap_or_else(|_| Value::Array(Vec::new())),
        Err(_) => Value::Array(Vec::new()),
    }
}

/// Remove one Stage registration by handle id (F1, F2, …). Does not renumber remaining ids.
pub fn unstage_entry(session_id: &str, staged_id: &str) -> Result<Session, String> {
    let session_id = session_id.trim();
    let staged_id = staged_id.trim();
    if session_id.is_empty() {
        return Err("Missing session_id".into());
    }
    if staged_id.is_empty() {
        return Err("Missing staged id".into());
    }
    let mut session = load_session(session_id)?;
    let before = session.staged.len();
    session.staged.retain(|entry| entry.id != staged_id);
    if session.staged.len() == before {
        return Err("staged id not found".into());
    }
    save_session(&session)?;
    Ok(session)
}

pub fn append_turn(session_id: &str, turn: Turn) -> Result<Session, String> {
    let mut session = load_session(session_id)?;
    session.turns.push(turn);
    save_session(&session)?;
    Ok(session)
}
