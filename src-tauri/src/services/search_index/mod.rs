//! In-process search index: SQLite FTS5 trigram + CJK bigram.

mod bigram;
mod chunk;
mod chunk_rows;
mod collect;
mod query;
mod store;
mod sync;

pub use bigram::cjk_bigram_text;
pub use chunk::{chunk_markdown, MAX_CHUNK_CHARS, SHORT_TITLE_MERGE_CHARS};
pub use chunk_rows::{delete_ghosts, replace_path_chunks, upsert_chunks};
pub use collect::{
    collect_all, collect_knowledge, collect_notes, file_mtime_nanos,
    note_file_chunks, ChunkKey, SourceChunk,
};
pub use query::{
    cache_dir_or_err, classify_query, fold_by_doc_id, parse_limit, search, search_desktop_knowledge,
    search_desktop_spark, search_in_cache, QueryHit, QueryRoute, SearchFilter,
};
pub use store::{
    distinct_paths, existing_hash, index_exists, open, rebuild, upsert_document,
    IndexStats, INDEX_FILE, SCHEMA_VERSION,
};
pub use sync::{sync_note_files, sync_note_files_best_effort};

#[cfg(test)]
#[path = "../../unit-tests/services/search_index.rs"]
mod tests;
