use lulu_spark_lib::services::spark_data::{
    insert_knowledge_directory, insert_knowledge_doc, open_at, KnowledgeDirectoryRow, KnowledgeDocRow,
    DB_FILE, UNCATEGORIZED,
};
use tempfile::TempDir;

use crate::migration::run_migration;

fn temp_data() -> TempDir {
    TempDir::new().expect("tmp")
}

#[test]
fn populated_docs_keep_directory_id() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_knowledge_directory(
        &conn,
        &KnowledgeDirectoryRow::new("minted", "kept", UNCATEGORIZED, ""),
    )
    .expect("kept");
    insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k-kept", "minted", "docs/a.md"))
        .expect("k-kept");
    insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k-fresh", "fresh", "b.md")).expect("k-fresh");
    drop(conn);

    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.doc_count, 2);
}

#[test]
fn empty_library_is_ok() {
    let tmp = temp_data();
    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.doc_count, 0);
}
