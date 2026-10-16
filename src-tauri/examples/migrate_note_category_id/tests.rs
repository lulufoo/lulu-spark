use lulu_spark_lib::services::spark_data::{
    get_note, insert_note, insert_note_category, open_at, NoteCategoryRow, NoteRow, DB_FILE,
};
use tempfile::TempDir;

use crate::migration::run_migration;

fn temp_data() -> TempDir {
    TempDir::new().expect("tmp")
}

#[test]
fn path_first_segment_maps_to_category_id_without_changing_raw_path() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_note_category(&conn, &NoteCategoryRow::new("cat-ai", "ai", "AI", "")).expect("ai");
    insert_note_category(&conn, &NoteCategoryRow::new("ops", "ops", "Ops", "")).expect("ops");
    let mut ai_note = NoteRow::new("n-ai", "ai/theme/a.md");
    ai_note.category_id = "inbox".into();
    insert_note(&conn, &ai_note).expect("n-ai");
    insert_note(&conn, &NoteRow::new("n-ops", "ops/theme/b.md")).expect("n-ops");
    drop(conn);

    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.note_count, 2);

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let ai = get_note(&conn, "n-ai").expect("get").expect("n-ai");
    assert_eq!(ai.category_id, "cat-ai");
    assert_eq!(ai.raw_path, "ai/theme/a.md");
    let ops = get_note(&conn, "n-ops").expect("get").expect("n-ops");
    assert_eq!(ops.category_id, "ops");
    assert_eq!(ops.raw_path, "ops/theme/b.md");
}

#[test]
fn missing_folder_errors_and_writes_nothing() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_note(&conn, &NoteRow::new("n1", "ghost/theme/a.md")).expect("n1");
    drop(conn);

    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("ghost"), "{err}");

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let note = get_note(&conn, "n1").expect("get").expect("n1");
    assert_eq!(note.category_id, "inbox");
    assert_eq!(note.raw_path, "ghost/theme/a.md");
}

#[test]
fn empty_path_is_an_error() {
    let tmp = temp_data();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    insert_note(&conn, &NoteRow::new("n1", "")).expect("n1");
    drop(conn);

    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("empty path"), "{err}");
}
