//! One document service for every workspace: `workspace` + `project` locate a document,
//! the `workspace` registry resolves the document table and the folder on disk.
//!
//! This replaces the separate `notes` and `knowledge` services. Until the MCP tools and the
//! Tauri commands switch over (later issues), the old services keep serving the old tables.

mod create;
mod delete;
mod project;
mod read;
mod resolve;
mod update;
mod workspace;

pub use create::{create_doc, create_doc_content};
pub use delete::delete_doc;
pub use project::{create_project, delete_project, list_projects, update_project};
pub use read::{export_file, get_digest, get_file};
pub use resolve::DEFAULT_PROJECT_FOLDER;
pub use update::{update_doc, update_doc_content};
pub use workspace::{
    create_workspace, delete_workspace, list_workspaces, rename_workspace, set_view_mode,
};

#[cfg(test)]
#[path = "../../unit-tests/services/docs.rs"]
mod tests;
