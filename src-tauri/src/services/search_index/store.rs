//! Single-file FTS5 store: docs_tri (trigram) + docs_bi (unicode61 bigram).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use super::bigram::cjk_bigram_text;
use super::collect::SourceChunk;

pub const INDEX_FILE: &str = "search-index.sqlite";

/// Bump when table columns change; mismatched files are dropped and rebuilt.
pub const SCHEMA_VERSION: i32 = 2;

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
CREATE VIRTUAL TABLE IF NOT EXISTS docs_tri USING fts5(
  chunk_id UNINDEXED,
  doc_id UNINDEXED,
  category UNINDEXED,
  layer UNINDEXED,
  path UNINDEXED,
  common_path UNINDEXED,
  repo UNINDEXED,
  title,
  body,
  content_hash UNINDEXED,
  created_at UNINDEXED,
  mtime UNINDEXED,
  tokenize = 'trigram'
);
CREATE VIRTUAL TABLE IF NOT EXISTS docs_bi USING fts5(
  chunk_id UNINDEXED,
  doc_id UNINDEXED,
  category UNINDEXED,
  layer UNINDEXED,
  path UNINDEXED,
  common_path UNINDEXED,
  repo UNINDEXED,
  title UNINDEXED,
  body,
  orig UNINDEXED,
  content_hash UNINDEXED,
  created_at UNINDEXED,
  tokenize = 'unicode61'
);
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
        conn.execute_batch("DROP TABLE IF EXISTS docs_tri; DROP TABLE IF EXISTS docs_bi;")
            .map_err(|e| e.to_string())?;
    }
    conn.execute_batch(CREATE_TABLES).map_err(|e| e.to_string())?;
    if version != SCHEMA_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))
            .map_err(|e| e.to_string())?;
    }
    Ok(conn)
}

/// `(content_hash, mtime)` of an indexed chunk.
///
/// One-row lookup on an FTS5 `UNINDEXED` column is a table scan. Prefer
/// [`load_existing_hashes`] when checking many chunks (rebuild / upsert).
pub fn existing_hash(conn: &Connection, chunk_id: &str) -> Result<Option<(String, i64)>, String> {
    conn.query_row(
        "SELECT content_hash, mtime FROM docs_tri WHERE chunk_id = ?1",
        params![chunk_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// One scan of `docs_tri` → `chunk_id → (content_hash, mtime)`.
fn load_existing_hashes(conn: &Connection) -> Result<HashMap<String, (String, i64)>, String> {
    let mut stmt = conn
        .prepare("SELECT chunk_id, content_hash, mtime FROM docs_tri")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (id, hash, mtime) = row.map_err(|e| e.to_string())?;
        map.insert(id, (hash, mtime));
    }
    Ok(map)
}

fn delete_chunk(conn: &Connection, chunk_id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM docs_tri WHERE chunk_id = ?1", params![chunk_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM docs_bi WHERE chunk_id = ?1", params![chunk_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn delete_path_rows(conn: &Connection, path: &str) -> Result<usize, String> {
    let n = conn
        .execute("DELETE FROM docs_tri WHERE path = ?1", params![path])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM docs_bi WHERE path = ?1", params![path])
        .map_err(|e| e.to_string())?;
    Ok(n)
}

fn insert_chunk(conn: &Connection, chunk: &SourceChunk) -> Result<(), String> {
    conn.execute(
        "INSERT INTO docs_tri(chunk_id, doc_id, category, layer, path, common_path, repo, title, body, content_hash, created_at, mtime)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            chunk.chunk_id,
            chunk.doc_id,
            chunk.category,
            chunk.layer,
            chunk.path,
            chunk.common_path,
            chunk.repo,
            chunk.title,
            chunk.body,
            chunk.content_hash,
            chunk.created_at,
            chunk.mtime,
        ],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO docs_bi(chunk_id, doc_id, category, layer, path, common_path, repo, title, body, orig, content_hash, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            chunk.chunk_id,
            chunk.doc_id,
            chunk.category,
            chunk.layer,
            chunk.path,
            chunk.common_path,
            chunk.repo,
            chunk.title,
            cjk_bigram_text(&chunk.body),
            chunk.body,
            chunk.content_hash,
            chunk.created_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Upsert chunks in one transaction. Unchanged `content_hash` is skipped on both
/// tables; a newer file `mtime` is still recorded so grep prefiltering stays trusted.
pub fn upsert_chunks(conn: &Connection, chunks: &[SourceChunk]) -> Result<IndexStats, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let existing = load_existing_hashes(&tx)?;
    let mut stats = IndexStats::default();
    for chunk in chunks {
        match existing.get(&chunk.chunk_id) {
            Some((hash, mtime)) if hash == &chunk.content_hash => {
                if *mtime != chunk.mtime {
                    tx.execute(
                        "UPDATE docs_tri SET mtime = ?1 WHERE chunk_id = ?2",
                        params![chunk.mtime, chunk.chunk_id],
                    )
                    .map_err(|e| e.to_string())?;
                }
                stats.skipped += 1;
            }
            Some(_) => {
                delete_chunk(&tx, &chunk.chunk_id)?;
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

pub fn delete_ghosts(
    conn: &Connection,
    live_ids: &HashSet<String>,
    category: Option<&str>,
    repo: Option<&str>,
) -> Result<usize, String> {
    let mut ids = Vec::new();
    match (category, repo) {
        (Some(c), Some(r)) => {
            let mut stmt = conn
                .prepare("SELECT chunk_id FROM docs_tri WHERE category = ?1 AND repo = ?2")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![c, r], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for id in rows {
                ids.push(id.map_err(|e| e.to_string())?);
            }
        }
        (Some(c), None) => {
            let mut stmt = conn
                .prepare("SELECT chunk_id FROM docs_tri WHERE category = ?1")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![c], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for id in rows {
                ids.push(id.map_err(|e| e.to_string())?);
            }
        }
        (None, _) => {
            let mut stmt = conn
                .prepare("SELECT chunk_id FROM docs_tri")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for id in rows {
                ids.push(id.map_err(|e| e.to_string())?);
            }
        }
    }
    let gone: Vec<String> = ids
        .into_iter()
        .filter(|id| !live_ids.contains(id))
        .collect();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for id in &gone {
        delete_chunk(&tx, id)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(gone.len())
}

/// Replace every chunk stored under `path` with `chunks` (may be empty = delete).
/// Path-scoped so a shrinking document leaves no stale tail chunks.
pub fn replace_path_chunks(
    conn: &Connection,
    path: &str,
    chunks: &[SourceChunk],
) -> Result<IndexStats, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let deleted = delete_path_rows(&tx, path)?;
    for chunk in chunks {
        insert_chunk(&tx, chunk)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(IndexStats {
        upserted: chunks.len(),
        skipped: 0,
        deleted,
    })
}

/// Single-document write hook. No-op when the index file does not exist yet:
/// the startup / manual full rebuild covers that case, and creating a partial
/// file here would make `index_exists` lie.
pub fn upsert_document(cache_dir: &Path, chunks: &[SourceChunk]) -> Result<IndexStats, String> {
    if !index_exists(cache_dir) {
        return Ok(IndexStats::default());
    }
    let conn = open(cache_dir)?;
    let mut paths: Vec<&str> = chunks.iter().map(|c| c.path.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    let mut stats = IndexStats::default();
    for path in paths {
        let subset: Vec<SourceChunk> = chunks
            .iter()
            .filter(|c| c.path == path)
            .cloned()
            .collect();
        let s = replace_path_chunks(&conn, path, &subset)?;
        stats.upserted += s.upserted;
        stats.deleted += s.deleted;
    }
    Ok(stats)
}

pub fn rebuild(
    cache_dir: &Path,
    chunks: &[SourceChunk],
    category: Option<&str>,
    repo: Option<&str>,
) -> Result<IndexStats, String> {
    let conn = open(cache_dir)?;
    let mut stats = upsert_chunks(&conn, chunks)?;
    let live: HashSet<String> = chunks.iter().map(|c| c.chunk_id.clone()).collect();
    stats.deleted = delete_ghosts(&conn, &live, category, repo)?;
    Ok(stats)
}

pub fn distinct_paths(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT path FROM docs_tri")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut set = HashSet::new();
    for path in rows {
        set.insert(path.map_err(|e| e.to_string())?);
    }
    Ok(set)
}

/// `path → newest mtime` for every indexed file.
pub fn indexed_mtimes(conn: &Connection) -> Result<HashMap<String, i64>, String> {
    let mut stmt = conn
        .prepare("SELECT path, MAX(mtime) FROM docs_tri GROUP BY path")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (path, mtime) = row.map_err(|e| e.to_string())?;
        map.insert(path, mtime);
    }
    Ok(map)
}
