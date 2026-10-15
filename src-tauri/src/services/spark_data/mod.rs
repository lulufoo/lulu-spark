//! Durable metadata database `spark-data.sqlite` (design: Linear project 元数据库 spark-data).

mod db;
mod json_column;
mod knowledge_doc_row;
mod note_row;

pub use db::{db_path, open, open_at, DB_FILE};
pub use knowledge_doc_row::{
    delete_knowledge_doc, find_knowledge_doc_id, get_knowledge_doc, insert_knowledge_doc,
    update_knowledge_doc, KnowledgeDocRow, KnowledgeStatus,
};
pub use note_row::{
    delete_note, find_note_ids_by_path, get_note, insert_note, update_note, NoteRow,
};

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data.rs"]
mod tests;
