//! In-process search index: SQLite FTS5 trigram + CJK bigram.

mod bigram;
mod chunk;
mod chunk_rows;
mod collect;
mod collect_docs;
mod hit_meta;
mod query;
mod store;
mod sync;

pub use bigram::cjk_bigram_text;
pub use chunk::{chunk_markdown, MAX_CHUNK_CHARS, SHORT_TITLE_MERGE_CHARS};
pub use chunk_rows::{delete_ghosts, replace_doc_chunks, upsert_chunks};
pub use collect::{ChunkKey, SourceChunk, DOCS_CATEGORY};
pub use collect_docs::{collect_workspace_docs, workspace_doc_chunks};
pub use hit_meta::resolve_hits;
pub use query::{
    cache_dir_or_err, classify_query, fold_by_doc_id, parse_limit, search, search_desktop_docs,
    search_in_cache, QueryHit, QueryRoute, SearchFilter,
};
pub use store::{
    existing_hash, index_exists, indexed_doc_ids, open, rebuild, IndexStats, INDEX_FILE, SCHEMA_VERSION,
};
pub use sync::{sync_workspace_docs, sync_workspace_docs_best_effort};

#[cfg(test)]
#[path = "../../unit-tests/services/search_index.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/services/search_index_workspace.rs"]
mod workspace_tests;
