use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

use super::schema::{SESSION_SCHEMA, SESSION_STATUS_IDLE};
use super::turn_store;
use super::types::{Session, StagedEntry};

pub fn sidecar_paths(path: &PathBuf) -> [PathBuf; 2] {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let parent = path.parent();
    let wal = parent.map(|p| p.join(format!("{name}-wal")));
    let shm = parent.map(|p| p.join(format!("{name}-shm")));
    [
        wal.unwrap_or_else(|| PathBuf::from(format!("{name}-wal"))),
        shm.unwrap_or_else(|| PathBuf::from(format!("{name}-shm"))),
    ]
}

fn open(path: &PathBuf) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=OFF;")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(SESSION_SCHEMA)
        .map_err(|e| e.to_string())?;
    ensure_last_prompt_columns(&conn)?;
    Ok(conn)
}

fn ensure_last_prompt_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(meta)")
        .map_err(|e| e.to_string())?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if !names.iter().any(|name| name == "last_prompt_tokens") {
        conn.execute(
            "ALTER TABLE meta ADD COLUMN last_prompt_tokens INTEGER",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    if !names.iter().any(|name| name == "last_prompt_breakdown") {
        conn.execute(
            "ALTER TABLE meta ADD COLUMN last_prompt_breakdown TEXT",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn store_last_prompt(path: &PathBuf, tokens: i64, breakdown: &str) -> Result<(), String> {
    let conn = open(path)?;
    conn.execute(
        "UPDATE meta SET last_prompt_tokens = ?1, last_prompt_breakdown = ?2",
        params![tokens, breakdown],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_last_prompt_breakdown(path: &PathBuf) -> Result<Option<String>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let conn = open(path)?;
    conn.query_row(
        "SELECT last_prompt_breakdown FROM meta LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Session not found".to_string())
}

pub fn load_summary_bodies(path: &PathBuf) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let conn = open(path)?;
    let mut stmt = conn
        .prepare("SELECT body FROM summaries ORDER BY replaced_from_seq ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut bodies = Vec::new();
    for row in rows {
        bodies.push(row.map_err(|e| e.to_string())?);
    }
    Ok(bodies)
}

pub fn active_turn_seq_range(path: &PathBuf) -> Result<Option<(i64, i64)>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let conn = open(path)?;
    conn.query_row(
        "SELECT MIN(seq), MAX(seq) FROM model_turns",
        [],
        |row| {
            let from: Option<i64> = row.get(0)?;
            let to: Option<i64> = row.get(1)?;
            Ok(from.zip(to))
        },
    )
    .map_err(|e| e.to_string())
}

pub fn replace_turns_with_summary(
    path: &PathBuf,
    from_seq: i64,
    to_seq: i64,
    body: &str,
) -> Result<(), String> {
    let mut conn = open(path)?;
    let now = super::schema::unix_secs();
    let summary_id = format!("summary_{}", crate::services::id::random_entry_id());
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO summaries (summary_id, replaced_from_seq, replaced_to_seq, body, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![summary_id, from_seq, to_seq, body, now],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM model_turns WHERE seq BETWEEN ?1 AND ?2",
        params![from_seq, to_seq],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_last_prompt_tokens(path: &PathBuf) -> Result<Option<i64>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let conn = open(path)?;
    conn.query_row(
        "SELECT last_prompt_tokens FROM meta LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Session not found".to_string())
}

pub fn save(path: &PathBuf, session: &Session, title: &str, now: i64) -> Result<(), String> {
    let mut conn = open(path)?;
    let stored_steps = turn_store::load_steps(&conn)?;
    let stored_staged = load_staged(&conn)?;
    let created_at = conn
        .query_row("SELECT created_at FROM meta LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(now);

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    turn_store::sync(&tx, &stored_steps, &session.turns, now)?;
    sync_staged(&tx, &stored_staged, &session.staged)?;
    tx.execute(
        "INSERT INTO meta (session_id, created_at, updated_at, title, status)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(session_id) DO UPDATE SET
           updated_at = excluded.updated_at,
           title = excluded.title,
           status = excluded.status",
        params![session.session_id, created_at, now, title, SESSION_STATUS_IDLE],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load(path: &PathBuf) -> Result<Session, String> {
    if !path.is_file() {
        return Err("Session not found".into());
    }
    let conn = open(path)?;
    let session_id: String = conn
        .query_row("SELECT session_id FROM meta LIMIT 1", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(Session {
        session_id,
        turns: turn_store::load_turns(&conn)?,
        staged: load_staged(&conn)?,
    })
}

pub fn load_ui_messages(path: &PathBuf) -> Result<Vec<Value>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let conn = open(path)?;
    let mut stmt = conn
        .prepare("SELECT role, content FROM messages ORDER BY seq ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "role": row.get::<_, String>(0)?,
                "content": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

fn sync_staged(
    tx: &Transaction<'_>,
    stored: &[StagedEntry],
    current: &[StagedEntry],
) -> Result<(), String> {
    for entry in current {
        if stored.iter().any(|stored_entry| stored_entry == entry) {
            continue;
        }
        tx.execute(
            "INSERT INTO staged (staged_id, path, title, kind) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(staged_id) DO UPDATE SET
               path = excluded.path,
               title = excluded.title,
               kind = excluded.kind",
            params![entry.id, entry.path, entry.title, entry.kind],
        )
        .map_err(|e| e.to_string())?;
    }
    for entry in stored {
        if !current.iter().any(|current| current.id == entry.id) {
            tx.execute("DELETE FROM staged WHERE staged_id = ?1", params![entry.id])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn load_staged(conn: &Connection) -> Result<Vec<StagedEntry>, String> {
    let mut stmt = conn
        .prepare("SELECT staged_id, path, title, kind FROM staged ORDER BY rowid ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StagedEntry {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                kind: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}
