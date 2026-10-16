use lulu_spark_lib::services::spark_data::{
    get_knowledge_doc, insert_knowledge_directory, insert_knowledge_doc, open_at,
    KnowledgeDirectoryRow, KnowledgeDocRow, DB_FILE, UNCATEGORIZED,
};
use tempfile::TempDir;

use crate::migration::run_migration;

fn temp_data() -> TempDir {
    TempDir::new().expect("tmp")
}

#[test]
fn repo_folder_maps_to_directory_id_without_changing_path() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_knowledge_directory(
        &conn,
        &KnowledgeDirectoryRow::new("minted", "kept", UNCATEGORIZED, ""),
    )
    .expect("kept");
    insert_knowledge_directory(
        &conn,
        &KnowledgeDirectoryRow::new("fresh", "fresh", UNCATEGORIZED, ""),
    )
    .expect("fresh");
    let mut kept = KnowledgeDocRow::new("k-kept", "kept", "docs/a.md");
    kept.directory_id = "fresh".into();
    insert_knowledge_doc(&conn, &kept).expect("k-kept");
    insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k-fresh", "fresh", "b.md")).expect("k-fresh");
    drop(conn);

    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.doc_count, 2);

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let kept = get_knowledge_doc(&conn, "k-kept").expect("get").expect("k-kept");
    assert_eq!(kept.directory_id, "minted");
    assert_eq!(kept.path, "docs/a.md");
    let fresh = get_knowledge_doc(&conn, "k-fresh").expect("get").expect("k-fresh");
    assert_eq!(fresh.directory_id, "fresh");
    assert_eq!(fresh.path, "b.md");
}

#[test]
fn missing_folder_errors_and_writes_nothing() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_knowledge_directory(
        &conn,
        &KnowledgeDirectoryRow::new("demo", "demo", UNCATEGORIZED, ""),
    )
    .expect("demo");
    let mut doc = KnowledgeDocRow::new("k1", "ghost", "a.md");
    doc.directory_id = "demo".into();
    insert_knowledge_doc(&conn, &doc).expect("k1");
    drop(conn);

    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("ghost"), "{err}");

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let doc = get_knowledge_doc(&conn, "k1").expect("get").expect("k1");
    assert_eq!(doc.directory_id, "demo");
    assert_eq!(doc.repo, "ghost");
}

#[test]
fn empty_repo_is_an_error() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_knowledge_directory(
        &conn,
        &KnowledgeDirectoryRow::new("demo", "demo", UNCATEGORIZED, ""),
    )
    .expect("demo");
    let mut doc = KnowledgeDocRow::new("k1", "", "a.md");
    doc.directory_id = "demo".into();
    insert_knowledge_doc(&conn, &doc).expect("k1");
    drop(conn);

    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("empty repo"), "{err}");
}
