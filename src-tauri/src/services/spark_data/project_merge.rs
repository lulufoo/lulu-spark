//! Remove a project once its documents have been given to another project.

use rusqlite::{params, Connection};

use super::doc_row::{update_doc, DocRow};

/// In one transaction: write every row of `docs` (each already pointing at its new project
/// and paths), then delete the project row `project_id`. Any failure leaves the database as it
/// was. The built-in project is never deleted.
pub fn delete_project_after_merge(
    conn: &Connection,
    project_id: &str,
    docs: &[DocRow],
) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for doc in docs {
        if !update_doc(&tx, doc)? {
            return Err(format!("doc {} does not exist", doc.doc_id));
        }
    }
    let removed = tx
        .execute("DELETE FROM project WHERE id = ?1 AND is_default = 0", params![project_id])
        .map_err(|e| e.to_string())?;
    if removed == 0 {
        return Err(format!("project {project_id} does not exist or is built-in"));
    }
    tx.commit().map_err(|e| e.to_string())
}
