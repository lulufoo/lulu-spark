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
    ensure_column(&conn, "sessions", "llm", "TEXT")?;
    Ok(conn)
}

fn ensure_column(conn: &Connection, table: &str, column: &str, decl: &str) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if names.iter().any(|name| name == column) {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn upsert(
    path: &PathBuf,
    session_id: &str,
    title: &str,
    updated_at: i64,
    llm: &str,
) -> Result<(), String> {
    let conn = open(path)?;
    conn.execute(
        "INSERT INTO sessions (session_id, title, updated_at, status, llm)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(session_id) DO UPDATE SET
           title = excluded.title,
           updated_at = excluded.updated_at,
           status = excluded.status,
           llm = excluded.llm",
        params![session_id, title, updated_at, SESSION_STATUS_IDLE, llm],
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
            "SELECT session_id, title, updated_at, llm
             FROM sessions
             ORDER BY updated_at DESC, rowid DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![HOME_CHAT_LIST_LIMIT as i64], |row| {
            let llm: Option<String> = row.get(3)?;
            let llm = llm
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let mut item = json!({
                "session_id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "updated_at": row.get::<_, i64>(2)?,
            });
            if let Some(llm) = llm {
                item["llm"] = json!(llm);
            }
            Ok(item)
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}
