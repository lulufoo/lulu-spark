//! Durable metadata database `spark-data.sqlite` (design: Linear project 元数据库 spark-data).

mod backup;
mod db;
mod doc_annotation;
mod doc_entry;
mod doc_id;
mod doc_row;
mod doc_table;
mod drop_folder_step;
mod json_column;
mod legacy_rows_step;
mod project_merge;
mod project_row;
mod tag_rows;
mod workspace_row;

pub use backup::{backup_daily, backup_dir, backup_for_day, BACKUP_DIR};
pub use db::{db_path, open, open_at, DB_FILE};
pub use doc_row::{
    count_docs_by_project, count_docs_in_project, delete_doc, get_doc, insert_doc, list_docs, update_doc, DocRow,
};
pub use doc_table::{create_doc_table, doc_table_for_id, validate_doc_table_name, validate_workspace_id};
pub use doc_annotation::annotation_object;
pub use doc_id::{duplicate_doc_ids, is_valid_doc_id, new_doc_id, split_doc_id};
pub use doc_entry::{entry_from_doc, find_doc, list_doc_entries, translation_common_paths, FoundDoc};

pub use project_merge::delete_project_after_merge;
pub use project_row::{
    get_inbox_project, get_project, insert_project, list_projects, reorder_projects, update_project,
    ProjectRow,
};
pub use tag_rows::{
    delete_unused_tags, find_tag_key_by_value, has_tag, insert_tag, read_tag_registry, update_tag_value,
};
pub use workspace_row::{
    create_workspace, disable_workspace, discard_new_workspace, enable_workspace, get_workspace,
    get_workspace_by_name, list_all_workspaces, list_disabled_workspaces, list_workspaces,
    rename_workspace, reorder_workspaces, set_workspace_icon, set_workspace_view_mode, ViewMode, WorkspaceRow, DEFAULT_WORKSPACE_ID,
    KNOWLEDGE_WORKSPACE_ID,
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
