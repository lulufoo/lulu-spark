//! The `document.version` column: how files created before it are upgraded, and how `DocRow` reads and writes it.

use super::db::{open_with_steps, BASELINE_VERSION, MIGRATIONS};
use super::*;

use rusqlite::types::Value;
use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0)).expect("version")
}

/// (name, type, notnull, default) per column of `document`, in table order.
fn document_columns(conn: &Connection) -> Vec<(String, String, i64, Option<String>)> {
    conn.prepare("PRAGMA table_info(document)")
        .expect("pragma")
        .query_map([], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .expect("cols")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn has_version_column(conn: &Connection) -> bool {
    document_columns(conn).iter().any(|(name, ..)| name == "version")
}

/// The default workspace's inbox, by plain SQL so it works on a file of any version.
fn inbox_id(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM project WHERE is_default = 1", [], |r| r.get(0)).expect("inbox")
}

/// Rows written before the version column existed: no `version` in the statement.
fn seed_old_documents(conn: &Connection) {
    let inbox = inbox_id(conn);
    conn.execute(
        "INSERT INTO document(doc_id, raw_path, project_id, title, done, importance, tags, links)
         VALUES ('d1', 'a/one.md', ?1, 'One', 1, 'high', '[\"t1\"]', '[\"https://example.com\"]')",
        [&inbox],
    )
    .expect("d1");
    conn.execute("INSERT INTO document(doc_id, raw_path, project_id) VALUES ('d2', 'a/two.md', ?1)", [&inbox])
        .expect("d2");
    conn.execute(
        "INSERT INTO document(doc_id, raw_path, project_id, digest_path, source_type) VALUES ('d3', 'b/three.md', ?1, 'b/three.digest.md', 'web')",
        [&inbox],
    )
    .expect("d3");
}

/// Every column of `document` except `version`, so a test can see nothing else moved.
fn other_columns(conn: &Connection) -> Vec<Vec<Value>> {
    conn.prepare(
        "SELECT doc_id, raw_path, digest_path, source_type, created_at, importance, done, links, comments, tags,
                highlights, translations, title, project_id FROM document ORDER BY doc_id",
    )
    .expect("prepare")
    .query_map([], |r| (0..14).map(|i| r.get::<_, Value>(i)).collect::<Result<Vec<_>, _>>())
    .expect("query")
    .collect::<Result<_, _>>()
    .expect("rows")
}

fn versions(conn: &Connection) -> Vec<(String, i64)> {
    conn.prepare("SELECT doc_id, version FROM document ORDER BY doc_id")
        .expect("prepare")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn doc_in_inbox(conn: &Connection, doc_id: &str, version: i64) -> DocRow {
    let mut doc = DocRow::new(doc_id, "x/y.md", &inbox_id(conn));
    doc.version = version;
    doc
}

#[test]
fn a_file_from_before_the_column_keeps_every_row_and_every_version_is_zero() {
    let (_dir, path) = temp_db();
    // The file as it was before this column: the baseline plus the workspace description step.
    drop(open_with_steps(&path, &MIGRATIONS[..1]).expect("before the column"));
    let old = Connection::open(&path).expect("raw");
    assert_eq!(user_version(&old), BASELINE_VERSION as i64 + 1);
    assert!(!has_version_column(&old));
    seed_old_documents(&old);
    let before = other_columns(&old);
    assert_eq!(before.len(), 3);
    drop(old);

    let conn = open_at(&path).expect("upgrade");

    assert_eq!(user_version(&conn), BASELINE_VERSION as i64 + 2);
    assert_eq!(other_columns(&conn), before, "no row lost, no other value changed");
    assert_eq!(versions(&conn), [("d1".to_string(), 0), ("d2".to_string(), 0), ("d3".to_string(), 0)]);
    for id in ["d1", "d2", "d3"] {
        assert_eq!(get_doc(&conn, id).expect("get").expect("row").version, 0, "{id}");
    }
}

#[test]
fn a_baseline_file_goes_straight_to_the_latest_version() {
    let (_dir, path) = temp_db();
    drop(open_with_steps(&path, &[]).expect("baseline"));
    let old = Connection::open(&path).expect("raw");
    assert_eq!(user_version(&old), BASELINE_VERSION as i64);
    seed_old_documents(&old);
    drop(old);

    let conn = open_at(&path).expect("upgrade");

    assert_eq!(user_version(&conn), BASELINE_VERSION as i64 + 2);
    assert_eq!(versions(&conn).len(), 3);
    assert!(versions(&conn).iter().all(|(_, version)| *version == 0));
    let has_description = conn.prepare("SELECT description FROM workspace").is_ok();
    assert!(has_description, "the earlier step still runs");
}

#[test]
fn the_step_does_not_run_again_and_a_stored_version_survives_reopening() {
    let (_dir, path) = temp_db();
    drop(open_with_steps(&path, &MIGRATIONS[..1]).expect("before the column"));
    let old = Connection::open(&path).expect("raw");
    seed_old_documents(&old);
    drop(old);

    let conn = open_at(&path).expect("upgrade");
    let mut doc = get_doc(&conn, "d2").expect("get").expect("row");
    doc.version = 5;
    assert!(update_doc(&conn, &doc).expect("update"));
    drop(conn);

    let reopened = open_at(&path).expect("reopen");
    assert_eq!(user_version(&reopened), BASELINE_VERSION as i64 + 2);
    assert_eq!(get_doc(&reopened, "d2").expect("get").expect("row").version, 5);
    assert_eq!(get_doc(&reopened, "d1").expect("get").expect("row").version, 0);
}

#[test]
fn a_new_file_has_the_column_as_a_non_null_integer_that_defaults_to_zero() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");

    let column = document_columns(&conn).into_iter().find(|(name, ..)| name == "version").expect("column");
    assert_eq!(column, ("version".to_string(), "INTEGER".to_string(), 1, Some("0".to_string())));

    seed_old_documents(&conn); // a statement that never names the column gets the default
    assert_eq!(versions(&conn).len(), 3);
    assert!(versions(&conn).iter().all(|(_, version)| *version == 0));
}

#[test]
fn a_new_doc_row_starts_at_version_zero_and_reads_back_the_same() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    let doc = doc_in_inbox(&conn, "n1", 0);
    assert_eq!(DocRow::new("n1", "x/y.md", &doc.project_id).version, 0);

    insert_doc(&conn, &doc).expect("insert");

    assert_eq!(get_doc(&conn, "n1").expect("get"), Some(doc));
}

#[test]
fn version_is_written_by_insert_and_update_and_read_by_get_and_list() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    let workspace = get_default_workspace(&conn).expect("workspace");
    insert_doc(&conn, &doc_in_inbox(&conn, "n1", 7)).expect("insert 7");
    insert_doc(&conn, &doc_in_inbox(&conn, "n2", 0)).expect("insert 0");
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 7);

    let mut changed = get_doc(&conn, "n1").expect("get").expect("row");
    changed.version = 12;
    changed.title = Some("Renamed".into());
    assert!(update_doc(&conn, &changed).expect("update"));

    assert_eq!(get_doc(&conn, "n1").expect("get"), Some(changed));
    let listed: Vec<(String, i64)> =
        list_docs(&conn, &workspace.id).expect("list").into_iter().map(|d| (d.doc_id, d.version)).collect();
    assert_eq!(listed, [("n1".to_string(), 12), ("n2".to_string(), 0)]);
}

#[test]
fn the_largest_whole_number_a_browser_can_hold_is_stored_exactly() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    let top = 9_007_199_254_740_991; // 2^53 - 1, the largest version the cloud accepts without rounding
    insert_doc(&conn, &doc_in_inbox(&conn, "n1", top)).expect("insert");
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, top);
}

#[test]
fn a_negative_version_is_refused_and_the_stored_one_is_kept() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    assert!(insert_doc(&conn, &doc_in_inbox(&conn, "bad", -1)).is_err(), "insert with -1");
    assert!(get_doc(&conn, "bad").expect("get").is_none(), "nothing was stored");

    insert_doc(&conn, &doc_in_inbox(&conn, "n1", 3)).expect("insert");
    let mut doc = get_doc(&conn, "n1").expect("get").expect("row");
    doc.version = -1;
    assert!(update_doc(&conn, &doc).is_err(), "update to -1");
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 3);
    assert!(conn.execute("UPDATE document SET version = NULL WHERE doc_id = 'n1'", []).is_err(), "NOT NULL");
}
