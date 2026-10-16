use std::fs;
use std::path::Path;

use lulu_spark_lib::services::spark_data::{
    get_knowledge_directory_by_folder, insert_knowledge_directory, list_knowledge_directories,
    open_at, KnowledgeDirectoryRow, DB_FILE, UNCATEGORIZED,
};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::migration::run_migration;

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, serde_json::to_string_pretty(value).expect("json")).expect("write");
}

fn mkdir(path: &Path) {
    fs::create_dir_all(path).expect("mkdir");
}

fn fixture() -> TempDir {
    let tmp = TempDir::new().expect("tmp");
    let kb = tmp.path().join("knowledge");
    mkdir(&kb.join("android-dev-docs"));
    mkdir(&kb.join("tech-language-kotlin"));
    mkdir(&kb.join("ai-dev-docs"));
    write_json(
        &kb.join("categories.json"),
        &json!({
            "version": 1,
            "categories": [
                { "id": "uncategorized", "name": "Uncategorized" },
                { "id": "cat-tech", "name": "Tech" }
            ]
        }),
    );
    write_json(
        &kb.join("repos.json"),
        &json!({
            "version": 1,
            "repos": [
                {
                    "full_name": "lulufoo/android-dev-docs",
                    "description": "Android",
                    "category_id": "uncategorized"
                },
                {
                    "full_name": "lulufoo/tech-language-kotlin",
                    "description": "",
                    "category_id": "cat-tech"
                },
                {
                    "full_name": "lulufoo/ai-dev-docs",
                    "description": "deprecated",
                    "category_id": "uncategorized"
                },
                {
                    "full_name": "lulufoo/lulu-workflow-framework",
                    "description": "",
                    "category_id": "cat-tech"
                },
                {
                    "full_name": "lulufoo/ghost-topic",
                    "description": "no disk",
                    "category_id": "uncategorized"
                }
            ]
        }),
    );
    tmp
}

#[test]
fn imports_disk_topics_skips_deprecated_and_deletes_json() {
    let tmp = fixture();
    let kb = tmp.path().join("knowledge");
    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.row_count, 2);
    assert_eq!(outcome.skipped, 3);
    assert!(!kb.join("repos.json").exists());
    assert!(!kb.join("categories.json").exists());
    assert!(kb.join("ai-dev-docs").is_dir());

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let rows = list_knowledge_directories(&conn).expect("list");
    assert_eq!(rows.len(), 2);
    let android = get_knowledge_directory_by_folder(&conn, "android-dev-docs")
        .expect("get")
        .expect("android");
    assert_eq!(android.category, UNCATEGORIZED);
    assert_eq!(android.description, "Android");
    assert_eq!(android.id.len(), 23);
    let kotlin = get_knowledge_directory_by_folder(&conn, "tech-language-kotlin")
        .expect("get")
        .expect("kotlin");
    assert_eq!(kotlin.category, "Tech");
    assert!(get_knowledge_directory_by_folder(&conn, "ai-dev-docs")
        .expect("get")
        .is_none());
    assert!(get_knowledge_directory_by_folder(&conn, "lulu-workflow-framework")
        .expect("get")
        .is_none());
    assert!(get_knowledge_directory_by_folder(&conn, "ghost-topic")
        .expect("get")
        .is_none());
}

#[test]
fn missing_json_is_refused_and_writes_nothing() {
    let tmp = TempDir::new().expect("tmp");
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("repos.json"), "{err}");
    assert!(!tmp.path().join(DB_FILE).exists());
}

#[test]
fn a_field_mismatch_deletes_nothing() {
    let tmp = fixture();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_knowledge_directory(
        &conn,
        &KnowledgeDirectoryRow::new("d1", "android-dev-docs", UNCATEGORIZED, "other"),
    )
    .expect("seed clash");
    drop(conn);

    let json_path = tmp.path().join("knowledge/repos.json");
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("android-dev-docs"), "{err}");
    assert!(json_path.is_file());
}
