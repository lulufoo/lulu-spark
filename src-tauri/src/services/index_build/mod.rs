//! Keyword index builders (notes / knowledge → FTS5).

pub mod common;
pub mod knowledge;
pub mod workbench;

pub use knowledge::rebuild as rebuild_knowledge_index;
pub use workbench::full_rebuild as rebuild_workbench_index;
