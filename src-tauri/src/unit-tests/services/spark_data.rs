use super::db::{migrate, open_with_steps};
use super::*;

use rusqlite::Connection;
use serde_json::json;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn table_names(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .expect("prepare");
    stmt.query_map([], |r| r.get::<_, String>(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0)).expect("version")
}

#[test]
fn first_open_creates_three_tables_and_reopen_changes_nothing() {
    let (_dir, path) = temp_db();
    let first = open_at(&path).expect("first open");
    let tables = table_names(&first);
    assert_eq!(tables, vec!["knowledge_doc", "note", "tag"]);
    let version = user_version(&first);
    assert_eq!(version, 1);
    insert_note(&first, &NoteRow::new("n1", "a/b.md")).expect("seed");
    drop(first);

    let second = open_at(&path).expect("reopen");
    assert_eq!(table_names(&second), tables);
    assert_eq!(user_version(&second), version);
    assert!(get_note(&second, "n1").expect("get").is_some(), "reopen must keep data");
}

#[test]
fn open_uses_wal_and_foreign_keys() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).expect("mode");
    assert_eq!(mode, "wal");
    let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).expect("fk");
    assert_eq!(fk, 1);
}

#[test]
fn note_primary_key_rejects_duplicate_doc_id() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    insert_note(&conn, &NoteRow::new("n1", "a.md")).expect("first");
    assert!(insert_note(&conn, &NoteRow::new("n1", "b.md")).is_err());
}

#[test]
fn knowledge_doc_rejects_duplicate_id_and_duplicate_repo_path() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k1", "demo", "a.md")).expect("first");
    assert!(
        insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k1", "demo", "b.md")).is_err(),
        "primary key"
    );
    assert!(
        insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k2", "demo", "a.md")).is_err(),
        "UNIQUE(repo, path)"
    );
    insert_knowledge_doc(&conn, &KnowledgeDocRow::new("k3", "other", "a.md"))
        .expect("same path in another repo is fine");
}

#[test]
fn note_round_trips_every_column() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mut note = NoteRow::new("n1", "topic/202604242339-x.md");
    note.digest_path = Some("topic/202604242339-x.md".into());
    note.source_type = Some("summary".into());
    note.created_at = Some("202604242339".into());
    note.importance = Some("high".into());
    note.done = true;
    note.links = json!([{ "url": "https://example.com" }]);
    note.comments = json!([{ "id": "c1", "text": "好", "ts": "2026-10-16T00:00:00Z" }]);
    note.tags = json!(["rust", "sqlite"]);
    note.highlights = json!([{ "id": "h1", "text": "片段", "occurrence": 1 }]);
    note.translations = json!(["zh"]);
    note.title = Some("标题".into());
    insert_note(&conn, &note).expect("insert");
    assert_eq!(get_note(&conn, "n1").expect("get"), Some(note.clone()));

    note.done = false;
    note.importance = None;
    note.raw_path = "moved/x.md".into();
    note.tags = json!([]);
    assert!(update_note(&conn, &note).expect("update"));
    assert_eq!(get_note(&conn, "n1").expect("get"), Some(note));
    assert!(!update_note(&conn, &NoteRow::new("missing", "m.md")).expect("update missing"));
}

#[test]
fn note_lookup_by_path_and_delete() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    insert_note(&conn, &NoteRow::new("n2", "same.md")).expect("n2");
    insert_note(&conn, &NoteRow::new("n1", "same.md")).expect("n1");
    insert_note(&conn, &NoteRow::new("n3", "other.md")).expect("n3");
    assert_eq!(find_note_ids_by_path(&conn, "same.md").expect("find"), vec!["n1", "n2"]);
    assert!(find_note_ids_by_path(&conn, "none.md").expect("find").is_empty());
    assert!(delete_note(&conn, "n1").expect("delete"));
    assert!(!delete_note(&conn, "n1").expect("delete again"));
    assert!(get_note(&conn, "n1").expect("get").is_none());
}

#[test]
fn knowledge_doc_round_trip_lookup_and_delete() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mut doc = KnowledgeDocRow::new("k1", "demo", "docs/a.md");
    doc.comments = json!([{ "id": "c1", "text": "t", "ts": "x" }]);
    doc.links = json!([{ "url": "https://example.com" }]);
    insert_knowledge_doc(&conn, &doc).expect("insert");
    assert_eq!(get_knowledge_doc(&conn, "k1").expect("get"), Some(doc.clone()));
    assert_eq!(
        find_knowledge_doc_id(&conn, "demo", "docs/a.md").expect("find").as_deref(),
        Some("k1")
    );
    assert!(find_knowledge_doc_id(&conn, "demo", "nope.md").expect("find").is_none());

    doc.status = KnowledgeStatus::Missing;
    doc.path = "docs/b.md".into();
    assert!(update_knowledge_doc(&conn, &doc).expect("update"));
    assert_eq!(get_knowledge_doc(&conn, "k1").expect("get"), Some(doc));
    assert!(delete_knowledge_doc(&conn, "k1").expect("delete"));
    assert!(!delete_knowledge_doc(&conn, "k1").expect("delete again"));
}

#[test]
fn schema_rejects_bad_values() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mut note = NoteRow::new("n1", "a.md");
    note.importance = Some("urgent".into());
    assert!(insert_note(&conn, &note).is_err(), "importance outside high|medium|low");
    let mut note = NoteRow::new("n2", "a.md");
    note.tags = json!({ "not": "an array" });
    assert!(insert_note(&conn, &note).is_err(), "JSON columns must be arrays");
    assert!(
        conn.execute(
            "INSERT INTO knowledge_doc(doc_id, repo, path, status) VALUES ('k', 'r', 'p', 'gone')",
            []
        )
        .is_err(),
        "status outside active|missing"
    );
}

const STEP_A: &str = "CREATE TABLE a (id INTEGER); INSERT INTO a VALUES (1);";
const STEP_B: &str = "CREATE TABLE b (id INTEGER); INSERT INTO a VALUES (2);";

fn a_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM a", [], |r| r.get(0)).expect("count a")
}

#[test]
fn each_user_version_runs_only_the_remaining_steps() {
    let (_dir, path) = temp_db();
    // empty file → both steps
    let fresh = open_with_steps(&path, &[STEP_A, STEP_B]).expect("fresh");
    assert_eq!(user_version(&fresh), 2);
    assert_eq!(a_rows(&fresh), 2);
    drop(fresh);

    // already current → nothing runs again
    let current = open_with_steps(&path, &[STEP_A, STEP_B]).expect("current");
    assert_eq!(user_version(&current), 2);
    assert_eq!(a_rows(&current), 2);
    drop(current);

    // version 1 file → only the second step
    let (_dir2, path2) = temp_db();
    drop(open_with_steps(&path2, &[STEP_A]).expect("v1"));
    let upgraded = open_with_steps(&path2, &[STEP_A, STEP_B]).expect("v1 → v2");
    assert_eq!(user_version(&upgraded), 2);
    assert_eq!(a_rows(&upgraded), 2, "step A must not run twice");
    assert!(table_names(&upgraded).contains(&"b".to_string()));
}

#[test]
fn newer_file_is_refused_and_failed_step_leaves_version_unchanged() {
    let (_dir, path) = temp_db();
    drop(open_with_steps(&path, &[STEP_A, STEP_B]).expect("v2"));
    let err = open_with_steps(&path, &[STEP_A]).expect_err("file newer than build");
    assert!(err.contains("newer"), "{err}");

    let conn = Connection::open_in_memory().expect("mem");
    let broken = "CREATE TABLE c (id INTEGER); THIS IS NOT SQL;";
    assert!(migrate(&conn, &[STEP_A, broken]).is_err());
    assert_eq!(user_version(&conn), 1, "failed step must roll back and keep the version");
    assert!(!conn
        .prepare("SELECT 1 FROM sqlite_master WHERE name = 'c'")
        .expect("prepare")
        .exists([])
        .expect("exists"));
}

fn path_of(conn: &Connection, repo: &str, doc_id: &str) -> Option<String> {
    get_knowledge_doc(conn, doc_id).expect("get").filter(|d| d.repo == repo).map(|d| d.path)
}

#[test]
fn register_is_idempotent_and_reactivates_a_missing_record() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let id = register_knowledge_doc(&conn, "r", "a.md").expect("register");
    assert_eq!(register_knowledge_doc(&conn, "r", "a.md").expect("again"), id);
    let mut doc = get_knowledge_doc(&conn, &id).expect("get").expect("row");
    doc.status = KnowledgeStatus::Missing;
    update_knowledge_doc(&conn, &doc).expect("mark missing");
    assert_eq!(register_knowledge_doc(&conn, "r", "a.md").expect("revive"), id);
    let revived = get_knowledge_doc(&conn, &id).expect("get").expect("row");
    assert_eq!(revived.status, KnowledgeStatus::Active);
    assert_ne!(register_knowledge_doc(&conn, "other", "a.md").expect("other repo"), id);
}

#[test]
fn move_and_remove_treat_percent_and_underscore_literally() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let inside = register_knowledge_doc(&conn, "r", "100%_done/a.md").expect("inside");
    let lookalike = register_knowledge_doc(&conn, "r", "100xxdone/a.md").expect("lookalike");
    let other_repo = register_knowledge_doc(&conn, "s", "100%_done/a.md").expect("other repo");

    move_knowledge_paths(&conn, "r", "100%_done", "done").expect("move folder");
    assert_eq!(path_of(&conn, "r", &inside).as_deref(), Some("done/a.md"));
    assert_eq!(path_of(&conn, "r", &lookalike).as_deref(), Some("100xxdone/a.md"));
    assert_eq!(path_of(&conn, "s", &other_repo).as_deref(), Some("100%_done/a.md"));

    assert_eq!(remove_knowledge_paths(&conn, "r", "done").expect("remove folder"), 1);
    assert!(get_knowledge_doc(&conn, &inside).expect("get").is_none());
    assert!(get_knowledge_doc(&conn, &lookalike).expect("get").is_some());
}
