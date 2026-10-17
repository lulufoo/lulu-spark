//! Keyword index builder (every workspace's documents → FTS5).

pub mod common;
pub mod spark;

pub use spark::full_rebuild as rebuild_spark_index;
