//! The `document.version` column as every new file has it, and how `DocRow` reads and writes it.

use super::*;

use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
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

/// The default workspace's inbox, by plain SQL.
fn inbox_id(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM project WHERE is_default = 1", [], |r| r.get(0)).expect("inbox")
}

/// Rows whose statement never names `version`.
fn insert_documents_without_version(conn: &Connection) {
    let inbox = inbox_id(conn);
    for id in ["d1", "d2", "d3"] {
        conn.execute(
            "INSERT INTO document(doc_id, raw_path, project_id) VALUES (?1, 'a/x.md', ?2)",
            [id, inbox.as_str()],
        )
        .expect("insert");
    }
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
fn a_new_file_has_the_column_as_a_non_null_integer_that_defaults_to_zero() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");

    let column = document_columns(&conn).into_iter().find(|(name, ..)| name == "version").expect("column");
    assert_eq!(column, ("version".to_string(), "INTEGER".to_string(), 1, Some("0".to_string())));

    insert_documents_without_version(&conn); // a statement that never names the column gets the default
    assert_eq!(versions(&conn), [("d1".to_string(), 0), ("d2".to_string(), 0), ("d3".to_string(), 0)]);
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
fn version_is_written_by_insert_and_read_by_get_and_list_and_left_alone_by_update() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    let workspace = get_default_workspace(&conn).expect("workspace");
    insert_doc(&conn, &doc_in_inbox(&conn, "n1", 7)).expect("insert 7");
    insert_doc(&conn, &doc_in_inbox(&conn, "n2", 0)).expect("insert 0");
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 7);

    let mut changed = get_doc(&conn, "n1").expect("get").expect("row");
    changed.version = 12; // the row says 12, the update must not copy it
    changed.title = Some("Renamed".into());
    assert!(update_doc(&conn, &changed).expect("update"));

    changed.version = 7;
    assert_eq!(get_doc(&conn, "n1").expect("get"), Some(changed));
    let listed: Vec<(String, i64)> =
        list_docs(&conn, &workspace.id).expect("list").into_iter().map(|d| (d.doc_id, d.version)).collect();
    assert_eq!(listed, [("n1".to_string(), 7), ("n2".to_string(), 0)]);
}

#[test]
fn bump_adds_one_each_call_and_changes_nothing_else() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    insert_doc(&conn, &doc_in_inbox(&conn, "n1", 0)).expect("insert");
    insert_doc(&conn, &doc_in_inbox(&conn, "n2", 5)).expect("insert");
    let before = get_doc(&conn, "n1").expect("get").expect("row");

    assert!(bump_doc_version(&conn, "n1").expect("bump"));
    assert!(bump_doc_version(&conn, "n1").expect("bump"));

    assert_eq!(get_doc(&conn, "n1").expect("get"), Some(DocRow { version: 2, ..before }));
    assert_eq!(versions(&conn), [("n1".to_string(), 2), ("n2".to_string(), 5)], "the other row is not touched");
}

#[test]
fn an_update_from_a_row_read_before_a_bump_does_not_undo_the_bump() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    insert_doc(&conn, &doc_in_inbox(&conn, "n1", 0)).expect("insert");
    let mut stale = get_doc(&conn, "n1").expect("get").expect("row");

    assert!(bump_doc_version(&conn, "n1").expect("bump")); // another caller changes the files meanwhile
    stale.done = true;
    assert!(update_doc(&conn, &stale).expect("update"));

    let now = get_doc(&conn, "n1").expect("get").expect("row");
    assert_eq!((now.version, now.done), (1, true));
}

#[test]
fn set_writes_the_number_it_is_given_and_refuses_a_negative_one() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    insert_doc(&conn, &doc_in_inbox(&conn, "n1", 4)).expect("insert");

    assert!(set_doc_version(&conn, "n1", 9).expect("set up"));
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 9);
    assert!(set_doc_version(&conn, "n1", 2).expect("set down"), "the cloud's number may be lower too");
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 2);

    assert!(set_doc_version(&conn, "n1", -1).is_err());
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 2);
}

#[test]
fn bump_and_set_report_false_for_a_document_that_does_not_exist() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    assert!(!bump_doc_version(&conn, "nope").expect("bump"));
    assert!(!set_doc_version(&conn, "nope", 3).expect("set"));
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
    assert!(conn.execute("UPDATE document SET version = -1 WHERE doc_id = 'n1'", []).is_err(), "plain SQL to -1");
    assert_eq!(get_doc(&conn, "n1").expect("get").expect("row").version, 3);
    assert!(conn.execute("UPDATE document SET version = NULL WHERE doc_id = 'n1'", []).is_err(), "NOT NULL");
}
