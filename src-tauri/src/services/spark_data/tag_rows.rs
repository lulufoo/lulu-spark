//! The `tag(key, value)` table. Reference counts are never stored: they are counted
//! from `note.tags` when asked for.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};

const REFS: &str = "(SELECT COUNT(*) FROM note, json_each(note.tags) WHERE json_each.value = tag.key)";

/// `{ "keys": { key: { value, refs } } }`, the shape `/api/tags-registry` has always had.
pub fn read_tag_registry(conn: &Connection) -> Result<Value, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT key, value, {REFS} FROM tag ORDER BY key"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))
        .map_err(|e| e.to_string())?;
    let mut keys = Map::new();
    for row in rows {
        let (key, value, refs) = row.map_err(|e| e.to_string())?;
        keys.insert(key, json!({ "value": value, "refs": refs }));
    }
    Ok(json!({ "keys": keys }))
}

pub fn has_tag(conn: &Connection, key: &str) -> Result<bool, String> {
    conn.query_row("SELECT 1 FROM tag WHERE key = ?1", params![key], |_| Ok(()))
        .optional()
        .map(|found| found.is_some())
        .map_err(|e| e.to_string())
}

/// The key carrying `value`; when several do, the most referenced one.
pub fn find_tag_key_by_value(conn: &Connection, value: &str) -> Result<Option<String>, String> {
    conn.query_row(
        &format!("SELECT key FROM tag WHERE value = ?1 ORDER BY {REFS} DESC, key LIMIT 1"),
        params![value],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn insert_tag(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute("INSERT INTO tag(key, value) VALUES (?1, ?2)", params![key, value])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// `false` when the key does not exist.
pub fn update_tag_value(conn: &Connection, key: &str, value: &str) -> Result<bool, String> {
    conn.execute("UPDATE tag SET value = ?2 WHERE key = ?1", params![key, value])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}

/// Drop every tag no note references any more.
pub fn delete_unused_tags(conn: &Connection) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM tag WHERE key NOT IN (SELECT json_each.value FROM note, json_each(note.tags))",
        [],
    )
    .map_err(|e| e.to_string())
}
