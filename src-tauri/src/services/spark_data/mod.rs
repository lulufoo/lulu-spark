//! Durable metadata database `spark-data.sqlite` (design: Linear project 元数据库 spark-data).

mod backup;
mod db;
mod doc_annotation;
mod doc_entry;
mod doc_id;
mod doc_row;
mod json_column;
mod project_merge;
mod project_row;
mod tag_rows;
mod workspace_row;

pub use backup::{backup_daily, backup_dir, backup_for_day, BACKUP_DIR};
pub use db::{db_path, open, open_at, DB_FILE};
pub use doc_row::{
    count_docs_by_project, count_docs_in_project, delete_doc, get_doc, insert_doc, list_docs, update_doc, DocRow,
};
pub use doc_annotation::annotation_object;
pub use doc_id::{is_valid_doc_id, new_doc_id};
pub use doc_entry::{entry_from_doc, find_doc, list_doc_entries, translation_common_paths, FoundDoc};

pub use project_merge::delete_project_after_merge;
pub use project_row::{
    get_default_project, get_project, insert_default_project, insert_project, list_projects, mint_project_id,
    reorder_projects, update_project, ProjectRow,
};
pub use tag_rows::{
    delete_unused_tags, find_tag_key_by_value, has_tag, insert_tag, read_tag_registry, update_tag_value,
};
pub use workspace_row::{
    create_workspace, disable_workspace, discard_new_workspace, enable_workspace, get_default_workspace,
    get_workspace, get_workspace_by_name, list_all_workspaces, list_disabled_workspaces, list_workspaces,
    rename_workspace, reorder_workspaces, set_workspace_description, set_workspace_icon,
    set_workspace_view_mode, validate_workspace_id, ViewMode,
    WorkspaceRow,
};

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data_workspace.rs"]
mod workspace_tests;
