//! One document service for every workspace: `workspace` + `project` locate a document,
//! the `workspace` registry resolves the document table and the folder on disk.
//!
//! This replaces the separate `notes` and `knowledge` services. The shared file helpers
//! (`assets`, `create_meta`, `digest`, `store`, `jot`) and the row editor (`edit`) live here too.

pub(crate) mod assets;
pub(crate) mod create_meta;
mod create;
mod delete;
pub(crate) mod digest;
pub mod edit;
mod jot;
mod move_doc;
mod project;
mod read;
mod resolve;
mod search;
pub(crate) mod store;
mod update;
mod workspace;

pub use create::{create_doc, create_doc_content};
pub use delete::delete_doc;
pub use jot::*;
pub use move_doc::move_doc;
pub use project::{create_project, delete_project, list_projects, update_project};
pub use read::{export_file, get_digest, get_file, get_path};
pub(crate) use resolve::workspace_root;
pub use resolve::DEFAULT_PROJECT_FOLDER;
pub use search::search;
pub use store::{doc_file_path, workspace_dir};
pub use update::{update_doc, update_doc_content};
pub use workspace::{
    create_workspace, delete_workspace, list_workspaces, rename_workspace, set_view_mode,
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
