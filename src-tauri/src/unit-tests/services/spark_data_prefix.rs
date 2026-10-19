//! Migration step 16: prefix leftover `doc_id` values with the workspace id.

use super::db::{open_with_steps, MIGRATIONS};
use super::prefix_legacy_doc_ids::PREFIX_LEGACY_DOC_IDS_STEP;
use super::*;
use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn default_ws(conn: &Connection) -> WorkspaceRow {
    get_default_workspace(conn).expect("default workspace")
}

fn before_prefix() -> &'static [&'static str] {
    let i = MIGRATIONS
        .iter()
        .position(|s| *s == PREFIX_LEGACY_DOC_IDS_STEP)
        .expect("prefix step");
    &MIGRATIONS[..i]
}

const PLAIN: &str = "dddddddddddddddddddddddddddddddd";
const STALE_HEX: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const KEEP_HEX: &str = "ffffffffffffffffffffffffffffffff";

#[test]
fn plain_and_stale_prefixed_ids_become_the_current_workspace_prefix() {
    let (_dir, path) = temp_db();
    let before = open_with_steps(&path, before_prefix()).expect("before prefix");
    let default = default_ws(&before);
    insert_doc(&before, &default.doc_table, &DocRow::new(PLAIN, "a.md", "inbox")).expect("plain");
    insert_doc(
        &before,
        &default.doc_table,
        &DocRow::new(&format!("zzzzzz-{STALE_HEX}"), "b.md", "inbox"),
    )
    .expect("stale");
    let keep = format!("{}-{KEEP_HEX}", default.id);
    insert_doc(&before, &default.doc_table, &DocRow::new(&keep, "c.md", "inbox")).expect("canonical");
    drop(before);

    let conn = open_at(&path).expect("prefix");
    let default = default_ws(&conn);
    let want_plain = format!("{}-{PLAIN}", default.id);
    let want_stale = format!("{}-{STALE_HEX}", default.id);
    assert!(get_doc(&conn, &default.doc_table, &want_plain).expect("get").is_some());
    assert!(get_doc(&conn, &default.doc_table, PLAIN).expect("get").is_none());
    assert!(get_doc(&conn, &default.doc_table, &want_stale).expect("get").is_some());
    assert!(get_doc(&conn, &default.doc_table, &format!("zzzzzz-{STALE_HEX}")).expect("get").is_none());
    assert!(
        get_doc(&conn, &default.doc_table, &format!("{}-{KEEP_HEX}", default.id))
            .expect("get")
            .is_some(),
        "already-canonical ids stay"
    );

    assert_eq!(find_doc(&conn, &want_plain).expect("find").expect("new").doc.doc_id, want_plain);
    drop(conn);
    let again = open_at(&path).expect("reopen");
    assert_eq!(
        find_doc(&again, &want_plain).expect("find").expect("stable").doc.doc_id,
        want_plain,
        "prefix step is stable"
    );
}
