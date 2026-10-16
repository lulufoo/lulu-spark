use std::fs;

use super::*;
use crate::services::spark_data::{get_note, insert_note, open_at, NoteRow};
use crate::test_support::TestSandbox;

fn seeded_db(dir: &Path) -> Connection {
    let conn = open_at(&dir.join("live").join("spark-data.sqlite")).expect("open");
    insert_note(&conn, &NoteRow::new("n1", "a/b.md")).expect("seed");
    conn
}

fn snapshot_days(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn snapshot_holds_the_data_and_a_second_run_the_same_day_does_nothing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let conn = seeded_db(tmp.path());
    let dir = tmp.path().join("backups");
    let made = backup_for_day(&conn, &dir, "20261016", 7).expect("backup").expect("new snapshot");
    assert!(made.ends_with("spark-data-20261016.sqlite"));
    let copy = open_at(&made).expect("open snapshot");
    assert!(get_note(&copy, "n1").expect("get").is_some());
    drop(copy);
    assert_eq!(backup_for_day(&conn, &dir, "20261016", 7).expect("again"), None);
    assert_eq!(snapshot_days(&dir), vec!["spark-data-20261016.sqlite"], "no partial file left behind");
}

#[test]
fn only_the_newest_snapshots_are_kept_and_other_files_are_left_alone() {
    let tmp = tempfile::tempdir().expect("tmp");
    let conn = seeded_db(tmp.path());
    let dir = tmp.path().join("backups");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("notes-from-me.txt"), "keep").expect("write");
    for day in ["20261010", "20261011", "20261012", "20261013"] {
        backup_for_day(&conn, &dir, day, 3).expect("backup");
    }
    assert_eq!(
        snapshot_days(&dir),
        vec![
            "notes-from-me.txt",
            "spark-data-20261011.sqlite",
            "spark-data-20261012.sqlite",
            "spark-data-20261013.sqlite",
        ]
    );
}

#[test]
fn backup_daily_writes_under_the_data_directory() {
    let sandbox = TestSandbox::new();
    let made = backup_daily().expect("backup").expect("new snapshot");
    assert!(made.starts_with(backup_dir().expect("dir")));
    assert!(made.starts_with(paths::runtime_data_dir().expect("data")));
    sandbox.assert_not_prod_path(&made).expect("sandbox path");
}
