//! In-process keyword index: SQLite FTS5 trigram + CJK bigram.

mod bigram;
mod chunk;
mod collect;
mod grep_prefilter;
mod query;
mod store;
mod sync;

pub use bigram::cjk_bigram_text;
pub use chunk::{chunk_markdown, MAX_CHUNK_CHARS, SHORT_TITLE_MERGE_CHARS};
pub use collect::{
    collect_all, collect_knowledge, collect_knowledge_text, collect_notes, file_mtime_nanos,
    note_file_chunks, SourceChunk,
};
pub use grep_prefilter::{
    candidate_paths, extract_literals, indexed_paths, prefilter_or_none, should_scan_file,
    GrepPrefilter,
};
pub use query::{
    cache_dir_or_err, classify_query, fold_by_doc_id, parse_limit, search, search_desktop_knowledge,
    search_desktop_spark, search_in_cache, QueryHit, QueryRoute, SearchFilter,
};
pub use store::{
    delete_ghosts, distinct_paths, existing_hash, index_exists, indexed_mtimes, open, rebuild,
    replace_path_chunks, upsert_chunks, upsert_document, IndexStats, INDEX_FILE, SCHEMA_VERSION,
};
pub use sync::{sync_note_files, sync_note_files_best_effort};

#[cfg(test)]
#[path = "../../unit-tests/services/keyword_index.rs"]
mod tests;
