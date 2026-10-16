//! One run: map each note's path folder to `note_category.id`, then reconcile.

use std::path::Path;

use lulu_spark_lib::services::spark_data::{
    get_note_category_by_folder, list_notes, open_at, DB_FILE,
};
use rusqlite::{params, Connection};

pub struct Outcome {
    pub note_count: usize,
}

fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(DB_FILE)
}

fn path_folder(raw_path: &str) -> Result<&str, String> {
    raw_path
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("empty path `{raw_path}`"))
}

fn count(conn: &Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// Set `note.category_id` from `raw_path`'s first segment. A missing folder, a
/// null, or a dangling FK rolls the transaction back.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let conn = open_at(&db_path(data_dir))?;
    let notes = list_notes(&conn)?;
    let expected = notes.len();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for note in &notes {
        let folder = path_folder(&note.raw_path)
            .map_err(|e| format!("note {}: {e}", note.doc_id))?;
        let cat = get_note_category_by_folder(&tx, folder)?.ok_or_else(|| {
            format!("no category with folder `{folder}` (note {})", note.doc_id)
        })?;
        tx.execute(
            "UPDATE note SET category_id = ?1 WHERE doc_id = ?2",
            params![cat.id, note.doc_id],
        )
        .map_err(|e| e.to_string())?;
    }
    let nulls = count(&tx, "SELECT COUNT(*) FROM note WHERE category_id IS NULL")?;
    let dangling = count(
        &tx,
        "SELECT COUNT(*) FROM note WHERE category_id NOT IN (SELECT id FROM note_category)",
    )?;
    let row_count = count(&tx, "SELECT COUNT(*) FROM note")?;
    if nulls != 0 || dangling != 0 || row_count != expected as i64 {
        return Err(format!(
            "reconcile failed: nulls={nulls} dangling={dangling} count={row_count} expected={expected}; nothing was written"
        ));
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Outcome {
        note_count: expected,
    })
}
