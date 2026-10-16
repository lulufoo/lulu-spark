use lulu_spark_lib::services::spark_data::{
    get_note, insert_note, insert_note_category, open_at, NoteCategoryRow, NoteRow, DB_FILE,
};
use tempfile::TempDir;

use crate::migration::run_migration;

fn temp_data() -> TempDir {
    TempDir::new().expect("tmp")
}

#[test]
fn strips_matching_folder_and_leaves_disk_layout_untouched() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_note_category(&conn, &NoteCategoryRow::new("cat-ai", "ai", "AI", "")).expect("ai");
    let mut note = NoteRow::new("n-ai", "ai/theme/a.md");
    note.category_id = "cat-ai".into();
    note.digest_path = Some("ai/theme/a.md".into());
    insert_note(&conn, &note).expect("n-ai");
    drop(conn);

    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.path_count, 2);

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let got = get_note(&conn, "n-ai").expect("get").expect("n-ai");
    assert_eq!(got.raw_path, "theme/a.md");
    assert_eq!(got.digest_path.as_deref(), Some("theme/a.md"));
    assert_eq!(got.category_id, "cat-ai");
}

#[test]
fn first_segment_mismatch_writes_nothing() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_note_category(&conn, &NoteCategoryRow::new("ops", "ops", "Ops", "")).expect("ops");
    let mut note = NoteRow::new("n1", "ghost/theme/a.md");
    note.category_id = "ops".into();
    insert_note(&conn, &note).expect("n1");
    drop(conn);

    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("ghost"), "{err}");
    assert!(err.contains("ops"), "{err}");

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let got = get_note(&conn, "n1").expect("get").expect("n1");
    assert_eq!(got.raw_path, "ghost/theme/a.md");
}

#[test]
fn already_stripped_path_is_an_error() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let note = NoteRow::new("n1", "theme/a.md");
    insert_note(&conn, &note).expect("n1");
    drop(conn);

    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("does not match folder"), "{err}");

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let got = get_note(&conn, "n1").expect("get").expect("n1");
    assert_eq!(got.raw_path, "theme/a.md");
}
