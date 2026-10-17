//! Single-file index: plain `chunk` table (metadata, unique key) + FTS5 text tables
//! `docs_tri` (trigram) and `docs_bi` (unicode61 over CJK bigrams).
//!
//! `workspace_id` says which workspace a chunk belongs to, so a search can be limited to one.
//!
//! A chunk carries no path, repo or mtime: `doc_id` is the key into `spark-data.sqlite`,
//! which knows where the document lives.
//!
//! `chunk.id` is the `rowid` of the matching row in both FTS tables. The FTS tables
//! hold only `body`; everything else lives in `chunk`, where `(doc_id, layer, idx)`
//! is enforced unique by the database.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use super::chunk_rows::{delete_ghosts, upsert_chunks};
use super::collect::{ChunkKey, SourceChunk};

pub const INDEX_FILE: &str = "search-index.sqlite";

/// Bump when table columns change; mismatched files are dropped and rebuilt.
pub const SCHEMA_VERSION: i32 = 5;

#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub upserted: usize,
    pub skipped: usize,
    pub deleted: usize,
}

pub fn index_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(INDEX_FILE)
}

pub fn index_exists(cache_dir: &Path) -> bool {
    index_path(cache_dir).is_file()
}

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS chunk (
  id INTEGER PRIMARY KEY,
  doc_id TEXT NOT NULL,
  layer TEXT NOT NULL,
  idx INTEGER NOT NULL,
  category TEXT NOT NULL,
  workspace_id TEXT NOT NULL,
  title TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE (doc_id, layer, idx)
);
CREATE VIRTUAL TABLE IF NOT EXISTS docs_tri USING fts5(body, tokenize = 'trigram');
CREATE VIRTUAL TABLE IF NOT EXISTS docs_bi USING fts5(body, tokenize = 'unicode61');
"#;

pub fn open(cache_dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let conn = Connection::open(index_path(cache_dir)).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=OFF;")
        .map_err(|e| e.to_string())?;
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if version != SCHEMA_VERSION {
        conn.execute_batch(
            "DROP TABLE IF EXISTS docs_tri; DROP TABLE IF EXISTS docs_bi; DROP TABLE IF EXISTS chunk;",
        )
        .map_err(|e| e.to_string())?;
    }
    conn.execute_batch(CREATE_TABLES).map_err(|e| e.to_string())?;
    if version != SCHEMA_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))
            .map_err(|e| e.to_string())?;
    }
    Ok(conn)
}

/// `content_hash` of one indexed chunk, found through the unique index.
pub fn existing_hash(
    conn: &Connection,
    doc_id: &str,
    layer: &str,
    idx: usize,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT content_hash FROM chunk WHERE doc_id = ?1 AND layer = ?2 AND idx = ?3",
        params![doc_id, layer, idx as i64],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Full or scoped rebuild. A stored chunk that is not in `chunks` is deleted when it is
/// in `category` (any, when `None`) and its document is not in `keep_docs`.
pub fn rebuild(
    cache_dir: &Path,
    chunks: &[SourceChunk],
    category: Option<&str>,
    keep_docs: &HashSet<String>,
) -> Result<IndexStats, String> {
    let conn = open(cache_dir)?;
    let mut stats = upsert_chunks(&conn, chunks)?;
    let live: HashSet<ChunkKey> = chunks.iter().map(|c| c.to_key()).collect();
    stats.deleted = delete_ghosts(&conn, &live, category, keep_docs)?;
    Ok(stats)
}

/// Every `doc_id` that has at least one chunk.
pub fn indexed_doc_ids(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT doc_id FROM chunk")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut set = HashSet::new();
    for id in rows {
        set.insert(id.map_err(|e| e.to_string())?);
    }
    Ok(set)
}
