//! One document service for every workspace: `workspace` + `project` locate a document,
//! the `workspace` registry resolves the document table and the folder on disk.
//!
//! This replaces the separate `notes` and `knowledge` services. The shared file helpers
//! (`assets`, `create_meta`, `digest`, `store`, `jot`) and the row editor (`edit`) live here too.

pub(crate) mod assets;
pub(crate) mod categories;
pub(crate) mod create_meta;
mod create;
mod delete;
pub(crate) mod digest;
pub mod edit;
pub(crate) mod hide_patterns;
mod jot;
mod merge_plan;
mod move_doc;
mod project;
mod read;
mod resolve;
mod search;
pub(crate) mod store;
mod update;
mod viewer_state;
mod workspace;

pub use categories::{
    is_exposed, is_legal, load as load_doc_categories, save as save_doc_categories, DocCategory,
};
pub use create::{create_doc, create_doc_content};
pub use delete::delete_doc;
pub use edit::set_source_type;
pub use hide_patterns::{
    add_hide_pattern, list_hide_patterns, remove_hide_pattern, update_hide_pattern,
};
pub use jot::*;
pub use move_doc::move_doc;
pub use project::{create_project, list_projects, reorder_projects, update_project};
pub use read::{export_file, get_digest, get_file, get_path};
pub(crate) use resolve::workspace_root;
pub use search::search;
pub use store::{doc_file_path, workspace_dir};
pub use update::{update_doc, update_doc_content};
pub use viewer_state::{get_viewer_state, set_viewer_state};
pub use workspace::{
    create_workspace, disable_workspace, enable_workspace, list_disabled_workspaces, list_workspaces,
    rename_workspace, reorder_workspaces, set_icon, set_view_mode,
};

#[cfg(test)]
#[path = "../../unit-tests/services/docs.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/services/docs_create.rs"]
mod create_tests;

#[cfg(test)]
#[path = "../../unit-tests/services/docs_assets.rs"]
mod assets_tests;
