//! Write the plan into the database. Strict inserts: a clash is an error.

use lulu_spark_lib::services::spark_data::{
    directory_id_for_folder, get_knowledge_doc, insert_note, register_knowledge_doc,
    update_knowledge_doc,
};
use rusqlite::{params, Connection};
use serde_json::json;

use crate::plan::Plan;

pub fn write_plan(conn: &Connection, plan: &Plan) -> Result<(), String> {
    for note in &plan.notes {
        insert_note(conn, note).map_err(|e| format!("note {}: {e}", note.doc_id))?;
    }
    for (key, value) in &plan.tags {
        conn.execute("INSERT INTO tag(key, value) VALUES (?1, ?2)", params![key, value])
            .map_err(|e| format!("tag {key}: {e}"))?;
    }
    for item in &plan.knowledge {
        let directory_id = directory_id_for_folder(conn, &item.repo)?;
        let doc_id = register_knowledge_doc(conn, &directory_id, &item.path)?;
        let mut row = get_knowledge_doc(conn, &doc_id)?.ok_or("registered document vanished")?;
        let empty = json!([]);
        if row.comments != empty || row.links != empty || row.highlights != empty {
            return Err(format!("{}/{} already carries annotations in the database", item.repo, item.path));
        }
        row.comments = item.annotation.comments.clone();
        row.links = item.annotation.links.clone();
        row.highlights = item.annotation.highlights.clone();
        update_knowledge_doc(conn, &row)?;
    }
    Ok(())
}
