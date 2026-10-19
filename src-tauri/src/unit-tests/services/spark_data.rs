use super::db::{migrate, open_with_steps, BASELINE_VERSION, MIGRATIONS};
use super::*;

use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn names(conn: &Connection, kind: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1 AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .expect("prepare");
    stmt.query_map([kind], |r| r.get::<_, String>(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn table_names(conn: &Connection) -> Vec<String> {
    names(conn, "table")
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0)).expect("version")
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

fn default_ws(conn: &Connection) -> WorkspaceRow {
    get_default_workspace(conn).expect("default workspace")
}

/// A file that was never opened by this build, at the given `user_version`.
fn file_at_version(path: &std::path::Path, version: i64) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    let conn = Connection::open(path).expect("raw");
    conn.execute_batch(&format!("CREATE TABLE marker (id INTEGER); PRAGMA user_version = {version};"))
        .expect("version");
}

/// Everything in `dir`: file names with their bytes. A refused open must leave this unchanged.
fn snapshot(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .expect("read dir")
        .map(|e| {
            let e = e.expect("entry");
            (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).expect("read"))
        })
        .collect();
    files.sort();
    files
}

#[test]
fn the_baseline_is_version_18_and_nothing_follows_it_yet() {
    assert_eq!(BASELINE_VERSION, 18);
    assert!(MIGRATIONS.is_empty());
}

#[test]
fn an_empty_file_is_created_at_the_baseline_with_the_whole_schema() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("first open");
    assert_eq!(user_version(&conn), BASELINE_VERSION as i64);

    assert_eq!(table_names(&conn), ["document", "project", "tag", "workspace"]);
    assert_eq!(names(&conn, "index"), ["document_project_id", "project_one_default", "workspace_one_default"]);
    assert_eq!(
        names(&conn, "trigger"),
        vec!["project_keys_immutable", "workspace_default_undeletable", "workspace_keys_immutable"]
    );
    assert_eq!(count(&conn, "SELECT count(*) FROM tag"), 0);
}

#[test]
fn the_seed_is_default_and_its_inbox_only() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let workspaces = list_workspaces(&conn).expect("list");
    assert_eq!(workspaces.len(), 1);
    let default = &workspaces[0];
    assert_eq!((default.name.as_str(), default.is_default, default.view_mode), ("Default", true, ViewMode::Directory));
    assert_eq!(default.id.len(), 6);
    assert_ne!(default.id, "default", "the id is random, not a literal");
    assert!(get_workspace_by_name(&conn, "Knowledge").expect("lookup").is_none());

    let projects = list_projects(&conn, &default.id).expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!((projects[0].name.as_str(), projects[0].is_default), ("Inbox", true));
    assert_ne!(projects[0].id, "inbox", "the inbox id is minted like any project id");
    assert_eq!(projects[0].id.len(), 23, "YYYYMMDDHHMMSSmmm + 6 characters: {}", projects[0].id);
    assert_eq!(count(&conn, "SELECT count(*) FROM project"), 1);
}

#[test]
fn reopening_changes_nothing_and_keeps_rows() {
    let (_dir, path) = temp_db();
    let first = open_at(&path).expect("first open");
    let default = default_ws(&first);
    let tables = table_names(&first);
    let inbox = get_default_project(&first, &default.id).expect("lookup").expect("inbox");
    insert_doc(&first, &DocRow::new("n1", "a/b.md", &inbox.id)).expect("seed");
    drop(first);

    let second = open_at(&path).expect("reopen");
    assert_eq!(default_ws(&second).id, default.id, "the seeded id is stable");
    assert_eq!(table_names(&second), tables);
    assert_eq!(user_version(&second), BASELINE_VERSION as i64);
    let kept = get_doc(&second, "n1").expect("get").expect("row");
    assert_eq!(kept.project_id, inbox.id);
}

#[test]
fn a_file_below_the_baseline_is_refused_and_left_untouched() {
    for version in [1, 16, 17] {
        let (dir, path) = temp_db();
        file_at_version(&path, version);
        let before = snapshot(dir.path().join("data").as_path());
        let err = open_at(&path).expect_err("below the baseline");
        assert!(err.contains(&format!("version {version}")) && err.contains("not supported"), "{err}");
        assert_eq!(snapshot(dir.path().join("data").as_path()), before, "version {version}: file changed");
    }
}

#[test]
fn a_file_newer_than_the_build_is_refused_and_left_untouched() {
    let (dir, path) = temp_db();
    file_at_version(&path, BASELINE_VERSION as i64 + 1);
    let before = snapshot(dir.path().join("data").as_path());
    let err = open_at(&path).expect_err("newer than this build");
    assert!(err.contains("newer than this build"), "{err}");
    assert_eq!(snapshot(dir.path().join("data").as_path()), before);
}

#[test]
fn a_baseline_file_opens_without_being_written() {
    let (dir, path) = temp_db();
    drop(open_at(&path).expect("create"));
    let before = snapshot(dir.path().join("data").as_path());
    let conn = open_at(&path).expect("reopen");
    assert_eq!(user_version(&conn), BASELINE_VERSION as i64);
    drop(conn);
    assert_eq!(snapshot(dir.path().join("data").as_path()), before);
}

#[test]
fn a_baseline_file_without_a_default_workspace_gets_one_and_an_existing_one_is_kept() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("create");
    let old = default_ws(&conn);

    // Opening again keeps the one it has.
    drop(conn);
    let conn = open_at(&path).expect("reopen");
    assert_eq!(default_ws(&conn).id, old.id);
    assert_eq!(count(&conn, "SELECT count(*) FROM workspace"), 1);

    // Take Default away by hand (the guard trigger has to go first), at the same version.
    conn.execute_batch(
        "DROP TRIGGER workspace_default_undeletable;
         DELETE FROM project;
         DELETE FROM workspace;",
    )
    .expect("remove Default");
    assert_eq!(user_version(&conn), BASELINE_VERSION as i64);
    drop(conn);

    let healed = open_at(&path).expect("open without Default");
    let fresh = default_ws(&healed);
    assert_eq!((fresh.name.as_str(), fresh.is_default), ("Default", true));
    assert_eq!(count(&healed, "SELECT count(*) FROM workspace"), 1);
    assert_eq!(
        count(&healed, &format!("SELECT count(*) FROM project WHERE is_default = 1 AND workspace_id = '{}'", fresh.id)),
        1
    );
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
    let inbox = get_default_project(&conn, &default_ws(&conn).id).expect("lookup").expect("inbox");
    insert_doc(&conn, &DocRow::new("ok", "a.md", &inbox.id)).expect("a valid row goes in");
    let mut doc = DocRow::new("n1", "a.md", &inbox.id);
    doc.importance = Some("urgent".into());
    assert!(insert_doc(&conn, &doc).is_err(), "importance outside high|medium|low");
    let mut doc = DocRow::new("n2", "a.md", &inbox.id);
    doc.tags = serde_json::json!({ "not": "an array" });
    assert!(insert_doc(&conn, &doc).is_err(), "JSON columns must be arrays");
    assert!(
        insert_doc(&conn, &DocRow::new("n3", "a.md", "missing")).is_err(),
        "FK to project(id)"
    );
    assert!(insert_doc(&conn, &DocRow::new("ok", "b.md", &inbox.id)).is_err(), "doc_id is the primary key");
}

#[test]
fn a_workspace_accepts_only_one_inbox_and_the_flag_cannot_change() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let mut extra = ProjectRow::new("i2", &default_ws(&conn).id, "Other", "");
    extra.is_default = true;
    assert!(insert_project(&conn, &extra).is_err(), "one inbox per workspace");
    assert!(
        conn.execute("UPDATE project SET is_default = 0 WHERE is_default = 1", []).is_err(),
        "is_default is immutable"
    );
    assert!(
        conn.execute("UPDATE project SET id = 'moved' WHERE is_default = 1", []).is_err(),
        "id is immutable"
    );
    extra.is_default = false;
    insert_project(&conn, &extra).expect("a non-inbox project is fine");
}

// Upgrade machinery, exercised with test-only steps after the baseline.

const STEP_A: &str = "CREATE TABLE a (id INTEGER); INSERT INTO a VALUES (1);";
const STEP_B: &str = "CREATE TABLE b (id INTEGER); INSERT INTO a VALUES (2);";

fn a_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM a", [], |r| r.get(0)).expect("count a")
}

#[test]
fn each_user_version_runs_only_the_remaining_steps() {
    let base = BASELINE_VERSION as i64;

    // empty file → baseline, then both steps
    let (_dir, path) = temp_db();
    let fresh = open_with_steps(&path, &[STEP_A, STEP_B]).expect("fresh");
    assert_eq!(user_version(&fresh), base + 2);
    assert_eq!(a_rows(&fresh), 2);
    assert!(table_names(&fresh).contains(&"workspace".to_string()), "the baseline is there too");
    drop(fresh);

    // already latest → nothing runs again
    let current = open_with_steps(&path, &[STEP_A, STEP_B]).expect("current");
    assert_eq!(user_version(&current), base + 2);
    assert_eq!(a_rows(&current), 2);
    drop(current);

    // baseline file → both steps; one step done → only the second
    let (_dir2, path2) = temp_db();
    drop(open_with_steps(&path2, &[]).expect("baseline"));
    drop(open_with_steps(&path2, &[STEP_A]).expect("18 → 19"));
    let upgraded = open_with_steps(&path2, &[STEP_A, STEP_B]).expect("19 → 20");
    assert_eq!(user_version(&upgraded), base + 2);
    assert_eq!(a_rows(&upgraded), 2, "step A must not run twice");
    assert!(table_names(&upgraded).contains(&"b".to_string()));
}

#[test]
fn a_file_newer_than_the_step_list_is_refused() {
    let (_dir, path) = temp_db();
    drop(open_with_steps(&path, &[STEP_A, STEP_B]).expect("latest + 2"));
    let err = open_with_steps(&path, &[STEP_A]).expect_err("file newer than build");
    assert!(err.contains("newer"), "{err}");
}

#[test]
fn a_failed_step_rolls_back_and_leaves_the_version_unchanged() {
    let conn = Connection::open_in_memory().expect("mem");
    let broken = "CREATE TABLE c (id INTEGER); THIS IS NOT SQL;";
    assert!(migrate(&conn, &[STEP_A, broken]).is_err());
    assert_eq!(user_version(&conn), BASELINE_VERSION as i64 + 1, "the failed step must not move the version");
    assert!(table_names(&conn).contains(&"a".to_string()), "the earlier step stays");
    assert!(!table_names(&conn).contains(&"c".to_string()), "the failed step must roll back");
}

/// Connections that open the same file at the same moment must create the baseline once and
/// run every step once.
fn open_concurrently(path: &std::path::Path, steps: &'static [&'static str], threads: usize) {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(threads));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let path = path.to_path_buf();
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                open_with_steps(&path, steps).map(drop)
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("thread").expect("every concurrent open succeeds");
    }
}

#[test]
fn concurrent_opens_of_an_empty_file_create_it_once() {
    static STEPS: [&str; 2] = [STEP_A, STEP_B];
    for _ in 0..10 {
        let (_dir, path) = temp_db();
        open_concurrently(&path, &STEPS, 8);
        let conn = open_with_steps(&path, &STEPS).expect("check");
        assert_eq!(user_version(&conn), BASELINE_VERSION as i64 + 2);
        assert_eq!(a_rows(&conn), 2, "each step ran once");
        assert_eq!(count(&conn, "SELECT count(*) FROM workspace"), 1);
        assert_eq!(count(&conn, "SELECT count(*) FROM project"), 1);
    }
}

#[test]
fn concurrent_opens_of_a_baseline_file_run_each_step_once() {
    static STEPS: [&str; 2] = [STEP_A, STEP_B];
    for _ in 0..10 {
        let (_dir, path) = temp_db();
        drop(open_with_steps(&path, &[]).expect("baseline"));
        open_concurrently(&path, &STEPS, 8);
        let conn = open_with_steps(&path, &STEPS).expect("check");
        assert_eq!(user_version(&conn), BASELINE_VERSION as i64 + 2);
        assert_eq!(a_rows(&conn), 2, "each step ran once");
    }
}

/// Manual acceptance (LW-440): `SPARK_DB_COPY=/path/to/copy cargo test --lib migrate_real_copy -- --ignored`.
#[test]
#[ignore]
fn migrate_real_copy() {
    let path = std::env::var("SPARK_DB_COPY").expect("SPARK_DB_COPY");
    let conn = open_at(std::path::Path::new(&path)).expect("migrate");
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, (BASELINE_VERSION + MIGRATIONS.len()) as i64);
}
