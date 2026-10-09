use std::fs;
use std::path::PathBuf;

use chrono::{Local, TimeZone};
use serde_json::Value;

use crate::config::paths;
use crate::services::id::random_entry_id;

use super::catalog;
use super::schema::{checked_session_id, unix_secs};
use super::session_db;
use super::types::{Session, Step};

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

/// Per-request LLM call records of one session. Not under `agent-scratch` (model-writable).
pub fn session_llm_calls_dir(session_id: &str) -> Result<PathBuf, String> {
    let id = checked_session_id(session_id)?;
    Ok(sessions_dir()?.join(id).join("llm-calls"))
}

fn catalog_file_path() -> Result<PathBuf, String> {
    let _ = sessions_dir()?;
    Ok(catalog::catalog_path(agent_dir()?))
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
    let id = checked_session_id(session_id)?;
    Ok(sessions_dir()?.join(format!("{id}.sqlite")))
}

fn current_session_llm() -> Option<String> {
    crate::config::settings::load()
        .ok()
        .and_then(|settings| {
            crate::config::settings::normalize_engine_value(&settings.assistant_engine)
                .map(|id| id.to_string())
        })
}

pub fn create_session() -> Result<Session, String> {
    let session = Session {
        session_id: format!("spark_chat_{}", random_entry_id()),
        steps: Vec::new(),
        staged: Vec::new(),
        llm: current_session_llm(),
    };
    save_session(&session)?;
    Ok(session)
}

pub fn save_session(session: &Session) -> Result<(), String> {
    let path = session_file_path(&session.session_id)?;
    let now = unix_secs();
    let title = session_list_title(session, now);
    session_db::save(&path, session, &title, now)?;
    catalog::upsert(
        &catalog_file_path()?,
        &session.session_id,
        &title,
        now,
        session.llm.as_deref().unwrap_or(""),
    )?;
    Ok(())
}

pub fn load_session(session_id: &str) -> Result<Session, String> {
    session_db::load(&session_file_path(session_id)?)
}

pub fn store_last_prompt(session_id: &str, tokens: i64, breakdown: &str) -> Result<(), String> {
    session_db::store_last_prompt(&session_file_path(session_id)?, tokens, breakdown)
}

pub fn load_last_prompt_breakdown(session_id: &str) -> Result<Option<String>, String> {
    session_db::load_last_prompt_breakdown(&session_file_path(session_id)?)
}

pub fn load_last_prompt_tokens(session_id: &str) -> Result<Option<i64>, String> {
    session_db::load_last_prompt_tokens(&session_file_path(session_id)?)
}

pub fn load_summary_bodies(session_id: &str) -> Result<Vec<String>, String> {
    session_db::load_summary_bodies(&session_file_path(session_id)?)
}

pub fn active_turn_seq_range(session_id: &str) -> Result<Option<(i64, i64)>, String> {
    session_db::active_turn_seq_range(&session_file_path(session_id)?)
}

pub fn replace_turns_with_summary(
    session_id: &str,
    from_seq: i64,
    to_seq: i64,
    body: &str,
) -> Result<(), String> {
    session_db::replace_turns_with_summary(&session_file_path(session_id)?, from_seq, to_seq, body)
}

pub fn delete_session(session_id: &str) -> Result<(), String> {
    let path = session_file_path(session_id)?;
    if !path.is_file() {
        return Err("Session not found".into());
    }
    fs::remove_file(&path).map_err(|e| e.to_string())?;
    for sidecar in session_db::sidecar_paths(&path) {
        if sidecar.is_file() {
            let _ = fs::remove_file(&sidecar);
        }
    }
    catalog::remove(&catalog_file_path()?, checked_session_id(session_id)?)?;
    Ok(())
}

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
        .steps
        .iter()
        .find(|step| step.role == "user")
        .and_then(|step| step.content.as_deref())
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

/// Disk session catalog for the Home history list. Newest first.
pub fn list_session_summaries() -> Result<Vec<Value>, String> {
    catalog::list_recent(&catalog_file_path()?)
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
    match session_file_path(id).and_then(|path| session_db::load_ui_messages(&path)) {
        Ok(items) => Value::Array(items),
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

pub fn append_step(session_id: &str, step: Step) -> Result<Session, String> {
    let mut session = load_session(session_id)?;
    session.steps.push(step);
    save_session(&session)?;
    Ok(session)
}
