//! Shared document-file helpers (digest, assets, store, export) used by the docs service.

pub(crate) mod assets;
pub(crate) mod create_meta;
pub(crate) mod digest;
mod jot;
pub(crate) mod store;

pub use jot::*;
pub use store::{doc_file_path, workspace_dir};

