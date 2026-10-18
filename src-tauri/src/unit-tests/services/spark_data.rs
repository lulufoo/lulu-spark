use super::db::{migrate, open_with_steps, MIGRATIONS};
use super::*;

use rusqlite::Connection;

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

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

const LATEST: i64 = MIGRATIONS.len() as i64;

#[test]
fn first_open_creates_the_workspace_tables_and_no_legacy_ones() {
    let (_dir, path) = temp_db();
    let first = open_at(&path).expect("first open");
    let tables = table_names(&first);
    assert_eq!(tables, vec!["doc_default", "doc_knowledge", "project", "tag", "workspace"]);
    let version = user_version(&first);
    assert_eq!(version, LATEST);
    insert_doc(&first, "doc_default", &DocRow::new("n1", "a/b.md", "inbox")).expect("seed");
    drop(first);

    let second = open_at(&path).expect("reopen");
    assert_eq!(table_names(&second), tables);
    assert_eq!(user_version(&second), version);
    let kept = get_doc(&second, "doc_default", "n1").expect("get").expect("row");
    assert_eq!(kept.project_id, "inbox");
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
fn schema_rejects_bad_values() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mut doc = DocRow::new("n1", "a.md", "inbox");
    doc.importance = Some("urgent".into());
    assert!(insert_doc(&conn, "doc_default", &doc).is_err(), "importance outside high|medium|low");
    let mut doc = DocRow::new("n2", "a.md", "inbox");
    doc.tags = serde_json::json!({ "not": "an array" });
    assert!(insert_doc(&conn, "doc_default", &doc).is_err(), "JSON columns must be arrays");
    assert!(
        insert_doc(&conn, "doc_default", &DocRow::new("n3", "a.md", "missing")).is_err(),
        "FK to project(id)"
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

/// Every released `user_version` upgraded step by step, starting from a file that holds
/// legacy rows at version 1; the rows must arrive in the workspace tables and the legacy
/// tables must be gone at the end.
#[test]
fn full_migration_chain_from_the_earliest_version_to_the_latest() {
    let (_dir, path) = temp_db();
    let v1 = open_with_steps(&path, &MIGRATIONS[..1]).expect("v1");
    assert_eq!(user_version(&v1), 1);
    v1.execute("INSERT INTO note (doc_id, raw_path, title) VALUES ('n1', 'ai/x.md', 'X')", [])
        .expect("note");
    for (id, repo, file) in [("k1", "demo", "docs/a.md"), ("k2", "ai-dev-docs", "x.md")] {
        v1.execute(
            "INSERT INTO knowledge_doc (doc_id, repo, path) VALUES (?1, ?2, ?3)",
            [id, repo, file],
        )
        .expect("knowledge doc");
    }
    v1.execute("INSERT INTO tag (key, value) VALUES ('t1', 'ai')", []).expect("tag");
    drop(v1);

    for version in 2..=LATEST {
        let conn = open_with_steps(&path, &MIGRATIONS[..version as usize]).expect("next step");
        assert_eq!(user_version(&conn), version, "stopped at step {version}");
    }

    let conn = open_at(&path).expect("latest");
    assert_eq!(user_version(&conn), LATEST);
    assert_eq!(table_names(&conn), vec!["doc_default", "doc_knowledge", "project", "tag", "workspace"]);

    let note = get_doc(&conn, "doc_default", "n1").expect("get").expect("note copied");
    assert_eq!((note.raw_path.as_str(), note.project_id.as_str(), note.title.as_deref()), ("ai/x.md", "inbox", Some("X")));
    assert_eq!(get_project(&conn, "inbox").expect("get").expect("inbox").workspace_id, "default");

    let k1 = get_doc(&conn, "doc_knowledge", "k1").expect("get").expect("knowledge copied");
    assert_eq!((k1.raw_path.as_str(), k1.project_id.as_str()), ("docs/a.md", "knowledge:demo"));
    assert!(get_doc(&conn, "doc_knowledge", "k2").expect("get").is_none(), "retired repo stays out");
    assert_eq!(count(&conn, "SELECT count(*) FROM project WHERE workspace_id = 'knowledge'"), 1);
    assert_eq!(count(&conn, "SELECT count(*) FROM tag"), 1, "tags are untouched");
}

/// Running step 8 (drop the legacy tables) on a file that no longer has them must not fail.
#[test]
fn dropping_the_legacy_tables_is_repeatable() {
    let (_dir, path) = temp_db();
    drop(open_at(&path).expect("latest"));
    let conn = Connection::open(&path).expect("raw");
    conn.execute_batch(MIGRATIONS[8]).expect("step runs again");
}

/// Step 10 adds a nullable `icon` to `workspace` and `project`; rows made before it have none.
#[test]
fn the_icon_step_leaves_existing_rows_without_an_icon() {
    let (_dir, path) = temp_db();
    let before = open_with_steps(&path, &MIGRATIONS[..9]).expect("v9");
    assert_eq!(user_version(&before), 9);
    before
        .execute(
            "INSERT INTO project(id, workspace_id, folder, title) VALUES ('p9', 'default', 'old', 'Old')",
            [],
        )
        .expect("project at v9");
    drop(before);

    let conn = open_at(&path).expect("latest");
    assert_eq!(count(&conn, "SELECT count(*) FROM workspace WHERE icon IS NOT NULL"), 0);
    assert_eq!(count(&conn, "SELECT count(*) FROM project WHERE icon IS NOT NULL"), 0);
    assert_eq!(count(&conn, "SELECT count(*) FROM project WHERE id = 'p9'"), 1);
}

/// Step 11 adds `sort_index`: workspaces start numbered default first then by name, projects
/// by title inside their workspace, from 0 and without gaps.
#[test]
fn the_sort_step_numbers_existing_rows_in_the_old_order() {
    let (_dir, path) = temp_db();
    let before = open_with_steps(&path, &MIGRATIONS[..11]).expect("v11");
    assert_eq!(user_version(&before), 11);
    for (id, name) in [("zed", "Zed"), ("amy", "amy")] {
        before
            .execute(
                "INSERT INTO workspace(id, name, doc_table) VALUES (?1, ?2, 'doc_' || ?1)",
                [id, name],
            )
            .expect("workspace at v11");
    }
    for (id, folder, title) in [("p1", "z", "Zebra"), ("p2", "a", "apple"), ("p3", "b", "Banana")] {
        before
            .execute(
                "INSERT INTO project(id, workspace_id, folder, title) VALUES (?1, 'knowledge', ?2, ?3)",
                [id, folder, title],
            )
            .expect("project at v11");
    }
    drop(before);

    let conn = open_at(&path).expect("latest");
    let ids = |sql: &str| -> Vec<String> {
        conn.prepare(sql)
            .expect("prepare")
            .query_map([], |r| r.get::<_, String>(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows")
    };
    assert_eq!(
        ids("SELECT id FROM workspace ORDER BY sort_index"),
        vec!["default", "amy", "knowledge", "zed"]
    );
    assert_eq!(
        ids("SELECT CAST(sort_index AS TEXT) FROM workspace ORDER BY sort_index"),
        vec!["0", "1", "2", "3"]
    );
    assert_eq!(
        ids("SELECT id FROM project WHERE workspace_id = 'knowledge' ORDER BY sort_index"),
        vec!["p2", "p3", "p1"]
    );
    assert_eq!(
        ids("SELECT CAST(sort_index AS TEXT) FROM project WHERE workspace_id = 'knowledge' ORDER BY sort_index"),
        vec!["0", "1", "2"]
    );
}

/// Step 12 marks the seeded Default inbox and does not invent one for Knowledge.
/// Step 13 then drops `folder`; the latest file has `is_inbox` and no `folder`.
#[test]
fn the_inbox_step_marks_seeded_inbox_and_skips_knowledge() {
    let (_dir, path) = temp_db();
    let after_inbox = open_with_steps(&path, &MIGRATIONS[..13]).expect("v13");
    assert_eq!(
        count(&after_inbox, "SELECT count(*) FROM project WHERE id = 'inbox' AND workspace_id = 'default' AND is_inbox = 1"),
        1
    );
    assert_eq!(
        count(&after_inbox, "SELECT count(*) FROM project WHERE workspace_id = 'knowledge' AND is_inbox = 1"),
        0
    );
    drop(after_inbox);
    let conn = open_at(&path).expect("latest");
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(project)")
        .expect("pragma")
        .query_map([], |r| r.get(1))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert!(cols.contains(&"is_inbox".to_string()), "{cols:?}");
    assert!(!cols.contains(&"folder".to_string()), "{cols:?}");
}

/// Step 14 renames `project.title` to `name`; a row made before it keeps its text.
#[test]
fn the_rename_step_turns_project_title_into_name_and_keeps_rows() {
    let (_dir, path) = temp_db();
    let before = open_with_steps(&path, &MIGRATIONS[..14]).expect("v14");
    assert_eq!(user_version(&before), 14);
    before
        .execute("INSERT INTO project(id, workspace_id, title) VALUES ('p14', 'default', 'Old')", [])
        .expect("project at v14");
    drop(before);

    let conn = open_at(&path).expect("latest");
    assert_eq!(user_version(&conn), LATEST);
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(project)")
        .expect("pragma")
        .query_map([], |r| r.get(1))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert!(cols.contains(&"name".to_string()), "{cols:?}");
    assert!(!cols.contains(&"title".to_string()), "{cols:?}");
    assert_eq!(get_project(&conn, "p14").expect("get").expect("row").name, "Old");
}

#[test]
fn a_workspace_accepts_only_one_inbox_and_the_flag_cannot_change() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mut extra = ProjectRow::new("i2", "default", "Other", "");
    extra.is_inbox = true;
    assert!(insert_project(&conn, &extra).is_err(), "one inbox per workspace");
    assert!(
        conn.execute("UPDATE project SET is_inbox = 0 WHERE id = 'inbox'", []).is_err(),
        "is_inbox is immutable"
    );
    assert!(
        conn.execute("UPDATE project SET id = 'moved' WHERE id = 'inbox'", []).is_err(),
        "id is immutable"
    );
    extra.is_inbox = false;
    insert_project(&conn, &extra).expect("a non-inbox project is fine");
}

/// Manual acceptance (LW-440): `SPARK_DB_COPY=/path/to/copy cargo test --lib migrate_real_copy -- --ignored`.
#[test]
#[ignore]
fn migrate_real_copy() {
    let path = std::env::var("SPARK_DB_COPY").expect("SPARK_DB_COPY");
    let conn = open_at(std::path::Path::new(&path)).expect("migrate");
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, MIGRATIONS.len() as i64);
}
