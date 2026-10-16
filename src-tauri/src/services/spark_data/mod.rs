//! Durable metadata database `spark-data.sqlite` (design: Linear project 元数据库 spark-data).

mod backup;
mod db;
mod json_column;
mod knowledge_directory_row;
mod knowledge_doc_row;
mod knowledge_registry;
mod note_annotation;
mod note_category_row;
mod note_entry;
mod note_row;
mod tag_rows;

pub use backup::{backup_daily, backup_dir, backup_for_day, BACKUP_DIR};
pub use db::{db_path, open, open_at, DB_FILE};
pub use knowledge_directory_row::{
    clear_knowledge_category, delete_knowledge_directory, directory_folder, get_knowledge_directory,
    get_knowledge_directory_by_folder, insert_knowledge_directory, list_knowledge_category_names,
    list_knowledge_directories, rename_knowledge_category, update_knowledge_directory,
    KnowledgeDirectoryRow, UNCATEGORIZED,
};
pub use knowledge_doc_row::{
    active_knowledge_doc_ids, delete_knowledge_doc, knowledge_doc_ids_outside_repo, find_knowledge_doc_id, get_knowledge_doc, insert_knowledge_doc,
    update_knowledge_doc, KnowledgeDocRow, KnowledgeStatus,
};
pub use knowledge_registry::{
    move_knowledge_paths, reconcile_knowledge_repo, register_knowledge_doc, remove_knowledge_paths,
    ReconcileStats,
};
pub use note_annotation::annotation_object;
pub use note_category_row::{
    category_folder, delete_note_category, get_note_category, get_note_category_by_folder,
    insert_note_category, list_note_categories, update_note_category, NoteCategoryRow,
};
pub use note_entry::{entry_from_note, list_note_entries, translation_common_paths};
pub use note_row::{
    delete_note, find_note_ids_by_path, get_note, insert_note, list_notes, update_note, NoteRow,
};

pub use tag_rows::{
    delete_unused_tags, find_tag_key_by_value, has_tag, insert_tag, read_tag_registry, update_tag_value,
};

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data.rs"]
mod tests;
