//! Durable metadata database `spark-data.sqlite` (design: Linear project 元数据库 spark-data).

mod backup;
mod db;
mod doc_annotation;
mod doc_entry;
mod doc_row;
mod doc_table;
mod json_column;
mod legacy_rows_step;
mod project_row;
mod tag_rows;
mod workspace_row;

pub use backup::{backup_daily, backup_dir, backup_for_day, BACKUP_DIR};
pub use db::{db_path, open, open_at, DB_FILE};
pub use doc_row::{
    count_docs_in_project, delete_doc, get_doc, insert_doc, list_docs, update_doc, DocRow,
};
pub use doc_table::{create_doc_table, doc_table_for_id, validate_doc_table_name, validate_workspace_id};
pub use doc_annotation::annotation_object;
pub use doc_entry::{entry_from_doc, find_doc, list_doc_entries, project_folder, translation_common_paths, FoundDoc};

pub use project_row::{
    delete_project, get_project, get_project_by_folder, insert_project, list_projects,
    update_project, ProjectRow,
};
pub use tag_rows::{
    delete_unused_tags, find_tag_key_by_value, has_tag, insert_tag, read_tag_registry, update_tag_value,
};
pub use workspace_row::{
    create_workspace, delete_workspace, get_workspace, list_workspaces, rename_workspace,
    set_workspace_view_mode, ViewMode, WorkspaceRow, DEFAULT_WORKSPACE_ID, KNOWLEDGE_WORKSPACE_ID,
};

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data_workspace.rs"]
mod workspace_tests;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data_legacy_copy.rs"]
mod legacy_copy_tests;
