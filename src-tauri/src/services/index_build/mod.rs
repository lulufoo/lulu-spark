//! Keyword index builders (notes / knowledge → FTS5).

pub mod common;
pub mod knowledge;
pub mod spark;

pub use knowledge::rebuild as rebuild_knowledge_index;
pub use spark::full_rebuild as rebuild_spark_index;
