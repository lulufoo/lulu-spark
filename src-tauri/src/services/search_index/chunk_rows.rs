//! Row-level writes: one chunk lives in `chunk` plus the same `rowid` in both FTS tables.
//!
//! Every function that touches more than one table must run inside a transaction so
//! the FTS rows and the `chunk` row can never be left orphaned.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection};

use super::bigram::cjk_bigram_text;
use super::collect::{ChunkKey, SourceChunk};
use super::store::IndexStats;

/// What `chunk` currently stores for one key.
struct StoredChunk {
    id: i64,
    content_hash: String,
    category: String,
    title: String,
    created_at: String,
}

impl StoredChunk {
    /// Same body, but some metadata column is out of date.
    fn metadata_differs(&self, chunk: &SourceChunk) -> bool {
        self.category != chunk.category
            || self.title != chunk.title
            || self.created_at != chunk.created_at
    }
}

fn load_existing(conn: &Connection) -> Result<HashMap<ChunkKey, StoredChunk>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, doc_id, layer, idx, content_hash, category, title, created_at
             FROM chunk",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let key: ChunkKey = (
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? as usize,
            );
            Ok((
                key,
                StoredChunk {
                    id: row.get(0)?,
                    content_hash: row.get(4)?,
                    category: row.get(5)?,
                    title: row.get(6)?,
                    created_at: row.get(7)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (key, stored) = row.map_err(|e| e.to_string())?;
        map.insert(key, stored);
    }
    Ok(map)
}

fn delete_chunk(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM docs_tri WHERE rowid = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM docs_bi WHERE rowid = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM chunk WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn select_chunk_ids(
    conn: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> Result<Vec<i64>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(args, |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn insert_chunk(conn: &Connection, chunk: &SourceChunk) -> Result<(), String> {
    conn.execute(
        "INSERT INTO chunk(doc_id, layer, idx, category, title, content_hash, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            chunk.doc_id,
            chunk.layer,
            chunk.idx as i64,
            chunk.category,
            chunk.title,
            chunk.content_hash,
            chunk.created_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO docs_tri(rowid, body) VALUES (?1, ?2)",
        params![id, chunk.body],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO docs_bi(rowid, body) VALUES (?1, ?2)",
        params![id, cjk_bigram_text(&chunk.body)],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Upsert chunks in one transaction, keyed by `(doc_id, layer, idx)`. An unchanged
/// `content_hash` keeps the stored text; stale metadata (title, ...) is
/// refreshed in place so a moved or retitled document stays correct without a rewrite.
pub fn upsert_chunks(conn: &Connection, chunks: &[SourceChunk]) -> Result<IndexStats, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let existing = load_existing(&tx)?;
    let mut stats = IndexStats::default();
    for chunk in chunks {
        match existing.get(&chunk.to_key()) {
            Some(old) if old.content_hash == chunk.content_hash => {
                if old.metadata_differs(chunk) {
                    tx.execute(
                        "UPDATE chunk SET category = ?1, title = ?2, created_at = ?3 WHERE id = ?4",
                        params![chunk.category, chunk.title, chunk.created_at, old.id],
                    )
                    .map_err(|e| e.to_string())?;
                }
                stats.skipped += 1;
            }
            Some(old) => {
                delete_chunk(&tx, old.id)?;
                insert_chunk(&tx, chunk)?;
                stats.upserted += 1;
            }
            None => {
                insert_chunk(&tx, chunk)?;
                stats.upserted += 1;
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(stats)
}

/// Remove chunks (within `category` when given) whose key is not in `live`, except those
/// of a document in `keep_docs`.
pub fn delete_ghosts(
    conn: &Connection,
    live: &HashSet<ChunkKey>,
    category: Option<&str>,
    keep_docs: &HashSet<String>,
) -> Result<usize, String> {
    let (sql, args): (&str, Vec<&str>) = match category {
        Some(c) => ("SELECT id, doc_id, layer, idx FROM chunk WHERE category = ?1", vec![c]),
        None => ("SELECT id, doc_id, layer, idx FROM chunk", vec![]),
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows: Vec<(i64, ChunkKey)> = stmt
        .query_map(rusqlite::params_from_iter(args), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? as usize,
                ),
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    let gone: Vec<i64> = rows
        .into_iter()
        .filter(|(_, key)| !live.contains(key) && !keep_docs.contains(&key.0))
        .map(|(id, _)| id)
        .collect();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for id in &gone {
        delete_chunk(&tx, *id)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(gone.len())
}

/// Replace every chunk of `doc_id` with `chunks` (may be empty = delete the document).
/// Document-scoped so a shrinking document leaves no stale tail chunks and a dropped
/// layer leaves no rows.
pub fn replace_doc_chunks(
    conn: &Connection,
    doc_id: &str,
    chunks: &[SourceChunk],
) -> Result<IndexStats, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let ids = select_chunk_ids(&tx, "SELECT id FROM chunk WHERE doc_id = ?1", params![doc_id])?;
    for id in &ids {
        delete_chunk(&tx, *id)?;
    }
    let own: Vec<&SourceChunk> = chunks.iter().filter(|c| c.doc_id == doc_id).collect();
    for chunk in &own {
        insert_chunk(&tx, chunk)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(IndexStats {
        upserted: own.len(),
        skipped: 0,
        deleted: ids.len(),
    })
}
