//! Migration step: give the seeded `default` / `knowledge` workspaces random ids.
//!
//! The historical seed still inserts those two rows so already-applied files keep
//! a stable checksum. This step mints `random_alnum6` ids, renames the document
//! tables and on-disk folders, and is a no-op when those ids are gone.
//!
//! A sidecar next to the database records the chosen ids so a crash between disk
//! and SQL retries with the same mapping instead of minting a second pair.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use super::doc_table::{doc_table_for_id, validate_doc_table_name, validate_workspace_id};
use super::drop_folder_step::run_with_foreign_keys_off;
use super::workspace_row::{get_workspace, mint_workspace_id};
use crate::config::paths;

/// Sentinel `MIGRATIONS` entry. Not executable SQL; `migrate` intercepts it.
pub(super) const REMAP_BUILTIN_WORKSPACE_IDS_STEP: &str =
    "-- remap built-in workspace ids default and knowledge";

const OLD_IDS: [&str; 2] = ["default", "knowledge"];
const MAP_FILE: &str = ".remap-builtin-workspace-ids";
const IMMUTABLE_TRIGGER: &str = r#"
CREATE TRIGGER workspace_keys_immutable BEFORE UPDATE OF id, doc_table, is_default ON workspace
BEGIN SELECT RAISE(ABORT, 'workspace id, doc_table and is_default cannot change'); END;
"#;

/// `(old id, new id)` for every seeded workspace that still uses a literal id.
pub(super) fn remap_builtin_workspace_ids(conn: &Connection, db_path: &Path) -> Result<(), String> {
    let Some(data_dir) = db_path.parent() else {
        return Err("spark-data.sqlite has no parent directory".to_string());
    };
    let planned = load_or_create_map(conn, data_dir)?;
    if planned.is_empty() {
        let _ = fs::remove_file(data_dir.join(MAP_FILE));
        return Ok(());
    }

    rename_workspace_dirs(data_dir, &planned)?;
    apply_db(conn, &planned)?;
    remap_cache_files(db_path, &planned)?;
    let _ = fs::remove_file(data_dir.join(MAP_FILE));
    Ok(())
}

fn load_or_create_map(conn: &Connection, data_dir: &Path) -> Result<Vec<(String, String)>, String> {
    let path = data_dir.join(MAP_FILE);
    if path.is_file() {
        return read_map(&path);
    }
    let mut planned = Vec::new();
    let mut taken = Vec::new();
    for old in OLD_IDS {
        if get_workspace(conn, old)?.is_none() {
            continue;
        }
        let new = mint_unused(conn, &taken)?;
        taken.push(new.clone());
        planned.push((old.to_string(), new));
    }
    if !planned.is_empty() {
        write_map(&path, &planned)?;
    }
    Ok(planned)
}

fn mint_unused(conn: &Connection, extra: &[String]) -> Result<String, String> {
    for _ in 0..8 {
        let id = mint_workspace_id(conn)?;
        if extra.iter().all(|taken| taken != &id) && OLD_IDS.iter().all(|old| *old != id) {
            return Ok(id);
        }
    }
    Err("could not mint a unique workspace id".to_string())
}

fn read_map(path: &Path) -> Result<Vec<(String, String)>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((old, new)) = line.split_once('=') else {
            return Err(format!("invalid remap map line: {line}"));
        };
        out.push((old.to_string(), new.to_string()));
    }
    Ok(out)
}

fn write_map(path: &Path, planned: &[(String, String)]) -> Result<(), String> {
    let mut body = String::new();
    for (old, new) in planned {
        body.push_str(old);
        body.push('=');
        body.push_str(new);
        body.push('\n');
    }
    fs::write(path, body).map_err(|e| e.to_string())
}

fn apply_db(conn: &Connection, planned: &[(String, String)]) -> Result<(), String> {
    run_with_foreign_keys_off(conn, || {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute_batch("DROP TRIGGER IF EXISTS workspace_keys_immutable")
            .map_err(|e| e.to_string())?;
        for (old, new) in planned {
            remap_one_workspace(&tx, old, new)?;
        }
        tx.execute_batch(IMMUTABLE_TRIGGER).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    })
}

fn remap_one_workspace(conn: &Connection, old: &str, new: &str) -> Result<(), String> {
    if get_workspace(conn, old)?.is_none() {
        return Ok(());
    }
    if get_workspace(conn, new)?.is_some() {
        return Err(format!("workspace {new} already exists"));
    }
    let new_table = doc_table_for_id(new)?;
    let old_table = doc_table_for_id(old)?;
    conn.execute(
        "UPDATE workspace SET id = ?1, doc_table = ?2 WHERE id = ?3",
        params![new, new_table, old],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE project SET workspace_id = ?1 WHERE workspace_id = ?2",
        params![new, old],
    )
    .map_err(|e| e.to_string())?;
    conn.execute_batch(&format!("ALTER TABLE {old_table} RENAME TO {new_table}"))
        .map_err(|e| e.to_string())?;
    rewrite_prefixed_doc_ids(conn, &new_table, old, new)?;
    Ok(())
}

/// `{old}-{hex}` → `{new}-{hex}`. Plain 32 hex rows stay for a later step.
fn rewrite_prefixed_doc_ids(conn: &Connection, table: &str, old: &str, new: &str) -> Result<(), String> {
    validate_doc_table_name(table)?;
    rewrite_prefixed_ids(
        conn,
        &format!("UPDATE {table} SET doc_id = ?1 || substr(doc_id, ?2) WHERE doc_id LIKE ?3"),
        old,
        new,
    )
}

fn rewrite_prefixed_ids(conn: &Connection, sql: &str, old: &str, new: &str) -> Result<(), String> {
    validate_workspace_id(old)?;
    validate_workspace_id(new)?;
    conn.execute(
        sql,
        params![new, (old.len() + 1) as i64, format!("{old}-%")],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn rename_workspace_dirs(data_dir: &Path, planned: &[(String, String)]) -> Result<(), String> {
    let root = data_dir.join("workspaces");
    for (old, new) in planned {
        rename_if_exists(&root.join(old), &root.join(new))?;
    }
    Ok(())
}

fn remap_cache_files(db_path: &Path, planned: &[(String, String)]) -> Result<(), String> {
    let Some(cache) = cache_dir_for_opened_db(db_path) else {
        return Ok(());
    };
    for (old, new) in planned {
        rename_if_exists(
            &cache.join("workspace-hide-patterns").join(format!("{old}.json")),
            &cache.join("workspace-hide-patterns").join(format!("{new}.json")),
        )?;
        rename_if_exists(
            &cache.join("workspace-viewer-state").join(format!("{old}.json")),
            &cache.join("workspace-viewer-state").join(format!("{new}.json")),
        )?;
        remap_search_index_workspace(&cache.join("search-index.sqlite"), old, new)?;
    }
    Ok(())
}

fn cache_dir_for_opened_db(opened_db: &Path) -> Option<PathBuf> {
    let prod = paths::spark_data_db_path().ok()?;
    let opened = opened_db.canonicalize().unwrap_or_else(|_| opened_db.to_path_buf());
    let prod = prod.canonicalize().unwrap_or(prod);
    (opened == prod).then(|| paths::cache_dir().ok())?
}

fn remap_search_index_workspace(index: &Path, old: &str, new: &str) -> Result<(), String> {
    if !index.is_file() {
        return Ok(());
    }
    let conn = Connection::open(index).map_err(|e| e.to_string())?;
    let has_chunk: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'chunk'")
        .and_then(|mut stmt| stmt.exists([]))
        .unwrap_or(false);
    if !has_chunk {
        return Ok(());
    }
    conn.execute(
        "UPDATE chunk SET workspace_id = ?1 WHERE workspace_id = ?2",
        params![new, old],
    )
    .map_err(|e| e.to_string())?;
    rewrite_prefixed_ids(
        &conn,
        "UPDATE chunk SET doc_id = ?1 || substr(doc_id, ?2) WHERE doc_id LIKE ?3",
        old,
        new,
    )?;
    Ok(())
}

fn rename_if_exists(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Ok(());
    }
    if to.exists() {
        return Err(format!(
            "refusing to rename {} onto existing {}",
            from.display(),
            to.display()
        ));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(from, to).map_err(|e| e.to_string())
}
