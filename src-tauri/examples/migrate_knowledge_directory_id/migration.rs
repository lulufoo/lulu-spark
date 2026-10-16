//! One run: map each knowledge doc's `repo` to `knowledge_directory.id`, then reconcile.

use std::path::Path;

use lulu_spark_lib::services::spark_data::{get_knowledge_directory_by_folder, open_at, DB_FILE};
use rusqlite::{params, Connection};

pub struct Outcome {
    pub doc_count: usize,
}

fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(DB_FILE)
}

fn repo_folder(repo: &str) -> Result<&str, String> {
    if repo.is_empty() {
        return Err("empty repo".into());
    }
    Ok(repo)
}

fn count(conn: &Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

fn list_docs(conn: &Connection) -> Result<Vec<(String, String)>, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id, repo FROM knowledge_doc")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Set `knowledge_doc.directory_id` from `repo` matching `knowledge_directory.folder`.
/// A missing folder, a blank, or a dangling FK rolls the transaction back.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let conn = open_at(&db_path(data_dir))?;
    let docs = list_docs(&conn)?;
    let expected = docs.len();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (doc_id, repo) in &docs {
        let folder = repo_folder(repo).map_err(|e| format!("doc {doc_id}: {e}"))?;
        let dir = get_knowledge_directory_by_folder(&tx, folder)?.ok_or_else(|| {
            format!("no directory with folder `{folder}` (doc {doc_id})")
        })?;
        tx.execute(
            "UPDATE knowledge_doc SET directory_id = ?1 WHERE doc_id = ?2",
            params![dir.id, doc_id],
        )
        .map_err(|e| e.to_string())?;
    }
    let blanks = count(
        &tx,
        "SELECT COUNT(*) FROM knowledge_doc WHERE directory_id IS NULL OR directory_id = ''",
    )?;
    let dangling = count(
        &tx,
        "SELECT COUNT(*) FROM knowledge_doc
         WHERE directory_id NOT IN (SELECT id FROM knowledge_directory)",
    )?;
    let row_count = count(&tx, "SELECT COUNT(*) FROM knowledge_doc")?;
    if blanks != 0 || dangling != 0 || row_count != expected as i64 {
        return Err(format!(
            "reconcile failed: blanks={blanks} dangling={dangling} count={row_count} expected={expected}; nothing was written"
        ));
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Outcome {
        doc_count: expected,
    })
}
