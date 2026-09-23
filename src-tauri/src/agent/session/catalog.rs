use std::path::PathBuf;

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use super::schema::{CATALOG_SCHEMA, SESSION_STATUS_IDLE};

const HOME_CHAT_LIST_LIMIT: usize = 20;

pub fn catalog_path(agent_dir: PathBuf) -> PathBuf {
    agent_dir.join("sessions-catalog.sqlite")
}

fn open(path: &PathBuf) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=OFF;")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(CATALOG_SCHEMA)
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn upsert(path: &PathBuf, session_id: &str, title: &str, updated_at: i64) -> Result<(), String> {
    let conn = open(path)?;
    conn.execute(
        "INSERT INTO sessions (session_id, title, updated_at, status)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(session_id) DO UPDATE SET
           title = excluded.title,
           updated_at = excluded.updated_at,
           status = excluded.status",
        params![session_id, title, updated_at, SESSION_STATUS_IDLE],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove(path: &PathBuf, session_id: &str) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let conn = open(path)?;
    conn.execute(
        "DELETE FROM sessions WHERE session_id = ?1",
        params![session_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_recent(path: &PathBuf) -> Result<Vec<Value>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let conn = open(path)?;
    let mut stmt = conn
        .prepare(
            "SELECT session_id, title, updated_at
             FROM sessions
             ORDER BY updated_at DESC, rowid DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![HOME_CHAT_LIST_LIMIT as i64], |row| {
            Ok(json!({
                "session_id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "updated_at": row.get::<_, i64>(2)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}
