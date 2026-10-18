use std::path::PathBuf;

use chrono::{Local, NaiveDate, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use super::schema::{CATALOG_SCHEMA, SESSION_STATUS_IDLE};

/// How many of the most recent calendar days that have chats the Home list reaches back to.
pub const RECENT_CHAT_DAYS: usize = 2;

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

/// Local midnight of `date` as Unix seconds. A zone that skips midnight (daylight saving) starts
/// the day at the first hour that exists.
fn local_midnight_secs(date: NaiveDate) -> Option<i64> {
    (0..=3u32).find_map(|hour| {
        let naive = date.and_hms_opt(hour, 0, 0)?;
        Local
            .from_local_datetime(&naive)
            .earliest()
            .map(|dt| dt.timestamp())
    })
}

/// The earliest moment the Home list reaches back to: local midnight of the `days`-th most recent
/// calendar day that has a chat, by `updated_at`. `0` (no cutoff) when fewer than `days` such days
/// exist, or when the catalog does not exist yet. Rows with `updated_at <= 0` have no day.
pub fn recent_cutoff(path: &PathBuf, days: usize) -> Result<i64, String> {
    if days == 0 || !path.is_file() {
        return Ok(0);
    }
    let conn = open(path)?;
    let mut stmt = conn
        .prepare("SELECT updated_at FROM sessions WHERE updated_at > 0 ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    let mut seen: Vec<NaiveDate> = Vec::with_capacity(days);
    for row in rows {
        let ts = row.map_err(|e| e.to_string())?;
        let Some(dt) = Local.timestamp_opt(ts, 0).single() else {
            continue;
        };
        let day = dt.date_naive();
        if seen.last() != Some(&day) {
            seen.push(day);
            if seen.len() == days {
                return Ok(local_midnight_secs(day).unwrap_or(0));
            }
        }
    }
    Ok(0)
}

/// Chats with `updated_at >= cutoff`, newest first. `cutoff = 0` returns every chat.
pub fn list_since(path: &PathBuf, cutoff: i64) -> Result<Vec<Value>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let conn = open(path)?;
    let mut stmt = conn
        .prepare(
            "SELECT session_id, title, updated_at, llm
             FROM sessions
             WHERE updated_at >= ?1
             ORDER BY updated_at DESC, rowid DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![cutoff], |row| {
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

/// Whether the chat has a catalog row with `updated_at < cutoff`, that is, it is outside the Home
/// list. A missing row or a missing catalog gives `false`.
pub fn is_older_than(path: &PathBuf, session_id: &str, cutoff: i64) -> Result<bool, String> {
    if !path.is_file() {
        return Ok(false);
    }
    let conn = open(path)?;
    let older: Option<bool> = conn
        .query_row(
            "SELECT updated_at < ?2 FROM sessions WHERE session_id = ?1",
            params![session_id, cutoff],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(older.unwrap_or(false))
}

/// Set one chat's `updated_at` and nothing else. A chat that is not in the catalog is left alone.
pub fn touch(path: &PathBuf, session_id: &str, updated_at: i64) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let conn = open(path)?;
    conn.execute(
        "UPDATE sessions SET updated_at = ?2 WHERE session_id = ?1",
        params![session_id, updated_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Escape `\`, `%` and `_` so a `LIKE ... ESCAPE '\'` pattern matches them as plain characters.
fn escape_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Chats whose title contains `query` (case-insensitive for ASCII letters), newest first, at most
/// `limit`, across the whole catalog. `%`, `_` and `\` in `query` are plain characters. An empty
/// (or all-blank) query, or a `limit` of 0, finds nothing.
pub fn search_titles(path: &PathBuf, query: &str, limit: usize) -> Result<Vec<Value>, String> {
    let query = query.trim();
    if query.is_empty() || limit == 0 || !path.is_file() {
        return Ok(Vec::new());
    }
    let pattern = format!("%{}%", escape_like(query));
    let conn = open(path)?;
    let mut stmt = conn
        .prepare(
            "SELECT session_id, title, updated_at
             FROM sessions
             WHERE title LIKE ?1 ESCAPE '\\'
             ORDER BY updated_at DESC, rowid DESC
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![pattern, limit as i64], |row| {
            Ok(json!({
                "session_id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "updated_at": row.get::<_, i64>(2)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut hits = Vec::new();
    for row in rows {
        hits.push(row.map_err(|e| e.to_string())?);
    }
    Ok(hits)
}
