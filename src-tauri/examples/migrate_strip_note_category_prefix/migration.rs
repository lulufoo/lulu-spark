//! One run: strip `folder/` from stored paths when it matches the note's category.

use std::path::Path;

use lulu_spark_lib::services::spark_data::{
    category_folder, list_notes, open_at, DB_FILE,
};
use rusqlite::{params, Connection};

pub struct Outcome {
    pub path_count: usize,
}

fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(DB_FILE)
}

fn count(conn: &Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// First segment must equal `folder`; the rest is the stored rel. Empty rest is an error.
fn strip_folder_prefix(path: &str, folder: &str, note_id: &str, field: &str) -> Result<String, String> {
    let (head, rest) = path.split_once('/').ok_or_else(|| {
        format!("note {note_id} {field}: `{path}` has no category prefix")
    })?;
    if head != folder {
        return Err(format!(
            "note {note_id} {field}: first segment `{head}` does not match folder `{folder}`"
        ));
    }
    if rest.is_empty() {
        return Err(format!("note {note_id} {field}: empty path after stripping `{folder}`"));
    }
    Ok(rest.to_string())
}

/// Strip the category folder from `raw_path` and `digest_path`. Does not move files.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let conn = open_at(&db_path(data_dir))?;
    let notes = list_notes(&conn)?;
    let expected = notes.len();
    let mut path_count = 0usize;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for note in &notes {
        let folder = category_folder(&tx, &note.category_id)
            .map_err(|e| format!("note {}: {e}", note.doc_id))?;
        let raw = strip_folder_prefix(&note.raw_path, &folder, &note.doc_id, "raw_path")?;
        path_count += 1;
        let digest = match note.digest_path.as_deref().filter(|p| !p.is_empty()) {
            Some(p) => {
                path_count += 1;
                Some(strip_folder_prefix(p, &folder, &note.doc_id, "digest_path")?)
            }
            None => None,
        };
        tx.execute(
            "UPDATE note SET raw_path = ?1, digest_path = ?2 WHERE doc_id = ?3",
            params![raw, digest, note.doc_id],
        )
        .map_err(|e| e.to_string())?;
    }
    let row_count = count(&tx, "SELECT COUNT(*) FROM note")?;
    if row_count != expected as i64 {
        return Err(format!(
            "reconcile failed: count={row_count} expected={expected}; nothing was written"
        ));
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Outcome { path_count })
}
