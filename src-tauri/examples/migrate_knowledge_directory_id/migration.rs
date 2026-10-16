//! One run: prove every knowledge doc has a live `directory_id`.

use std::path::Path;

use lulu_spark_lib::services::spark_data::{open_at, DB_FILE};
use rusqlite::Connection;

pub struct Outcome {
    pub doc_count: usize,
}

fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(DB_FILE)
}

fn count(conn: &Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// Reconcile `knowledge_doc.directory_id`: no blanks, no dangling FKs.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let conn = open_at(&db_path(data_dir))?;
    let blanks = count(
        &conn,
        "SELECT COUNT(*) FROM knowledge_doc WHERE directory_id IS NULL OR directory_id = ''",
    )?;
    let dangling = count(
        &conn,
        "SELECT COUNT(*) FROM knowledge_doc
         WHERE directory_id NOT IN (SELECT id FROM knowledge_directory)",
    )?;
    let row_count = count(&conn, "SELECT COUNT(*) FROM knowledge_doc")?;
    if blanks != 0 || dangling != 0 {
        return Err(format!(
            "reconcile failed: blanks={blanks} dangling={dangling} count={row_count}"
        ));
    }
    Ok(Outcome {
        doc_count: row_count as usize,
    })
}
