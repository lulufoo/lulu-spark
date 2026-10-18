//! Opening a chat refreshes its time only when it is outside the Home list (LW-461).

use rusqlite::{params, Connection};

use crate::agent::session;
use crate::commands::ai_assistant::select_chat_session_json;
use crate::test_support::TestSandbox;

fn open_catalog(sandbox: &TestSandbox) -> Connection {
    Connection::open(
        sandbox
            .cache_dir()
            .join("agent-exec")
            .join("sessions-catalog.sqlite"),
    )
    .expect("open catalog")
}

/// Unix seconds of a local wall-clock time. Fixed past dates keep the tests independent of today.
fn build_local_ts(y: i32, m: u32, d: u32, h: u32) -> i64 {
    use chrono::{Local, TimeZone};
    Local
        .with_ymd_and_hms(y, m, d, h, 0, 0)
        .earliest()
        .expect("local time exists")
        .timestamp()
}

fn seed_chat(sandbox: &TestSandbox, updated_at: i64) -> String {
    let id = session::create_session().expect("create").session_id;
    set_catalog_time(sandbox, &id, updated_at);
    id
}

fn set_catalog_time(sandbox: &TestSandbox, id: &str, updated_at: i64) {
    open_catalog(sandbox)
        .execute(
            "UPDATE sessions SET updated_at = ?2 WHERE session_id = ?1",
            params![id, updated_at],
        )
        .expect("set catalog time");
}

fn read_catalog_time(sandbox: &TestSandbox, id: &str) -> i64 {
    open_catalog(sandbox)
        .query_row(
            "SELECT updated_at FROM sessions WHERE session_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .expect("catalog row")
}

fn read_meta_time(id: &str) -> i64 {
    Connection::open(session::session_file_path(id).expect("path"))
        .expect("open session")
        .query_row("SELECT updated_at FROM meta LIMIT 1", [], |row| row.get(0))
        .expect("meta row")
}

fn list_ids() -> Vec<String> {
    session::list_session_summaries()
        .expect("list")
        .iter()
        .map(|s| s["session_id"].as_str().expect("id").to_string())
        .collect()
}

fn run_in_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    session::reset_live_for_tests();
    f(&sandbox);
    session::reset_live_for_tests();
}

#[test]
fn a_chat_inside_the_list_keeps_its_time_when_touched() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 9));
        let inside_ts = build_local_ts(2020, 3, 17, 12);
        let inside = seed_chat(sandbox, inside_ts);
        session::init_recent_cutoff().expect("cutoff");
        let meta_before = read_meta_time(&inside);

        session::touch_session(&inside).expect("touch");

        assert_eq!(read_catalog_time(sandbox, &inside), inside_ts, "catalog time is unchanged");
        assert_eq!(read_meta_time(&inside), meta_before, "session database time is unchanged");
    });
}

#[test]
fn opening_chats_back_and_forth_does_not_reorder_the_list() {
    run_in_sandbox(|sandbox| {
        let a = seed_chat(sandbox, build_local_ts(2020, 3, 18, 9));
        let b = seed_chat(sandbox, build_local_ts(2020, 3, 18, 8));
        let c = seed_chat(sandbox, build_local_ts(2020, 3, 17, 12));
        session::init_recent_cutoff().expect("cutoff");
        let order = vec![a.clone(), b.clone(), c.clone()];
        assert_eq!(list_ids(), order);

        for id in [&c, &a, &c, &b, &a, &b, &c] {
            select_chat_session_json(id).expect("select");
            assert_eq!(list_ids(), order, "the list order is stable after opening {id}");
        }
    });
}

#[test]
fn a_chat_outside_the_list_is_refreshed_once_and_then_stays_put() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 9));
        seed_chat(sandbox, build_local_ts(2020, 3, 17, 12));
        let old_ts = build_local_ts(2020, 3, 1, 12);
        let old = seed_chat(sandbox, old_ts);
        session::init_recent_cutoff().expect("cutoff");
        assert!(!list_ids().contains(&old));

        session::touch_session(&old).expect("first touch");
        let refreshed = read_catalog_time(sandbox, &old);
        assert!(refreshed > old_ts, "the first open refreshes it");
        assert_eq!(read_meta_time(&old), refreshed, "the session database agrees");
        assert_eq!(list_ids().first(), Some(&old), "it is now the newest in the list");

        // Move it to a known time inside the list; a second open must not change it again.
        let known = build_local_ts(2020, 3, 17, 13);
        set_catalog_time(sandbox, &old, known);
        session::touch_session(&old).expect("second touch");
        assert_eq!(read_catalog_time(sandbox, &old), known, "the second open writes nothing");
    });
}

#[test]
fn with_no_cutoff_opening_a_chat_never_refreshes_it() {
    run_in_sandbox(|sandbox| {
        // One active day: no cutoff, every chat is already in the list.
        let a_ts = build_local_ts(2020, 3, 18, 9);
        let a = seed_chat(sandbox, a_ts);
        let b = seed_chat(sandbox, build_local_ts(2020, 3, 18, 10));
        assert_eq!(session::init_recent_cutoff().expect("cutoff"), 0);
        session::touch_session(&a).expect("touch with cutoff 0");
        assert_eq!(read_catalog_time(sandbox, &a), a_ts);
        assert_eq!(list_ids(), vec![b.clone(), a.clone()]);

        // The cutoff not set yet behaves the same.
        session::reset_live_for_tests();
        session::touch_session(&a).expect("touch before the cutoff is set");
        assert_eq!(read_catalog_time(sandbox, &a), a_ts);
    });
}

#[test]
fn touching_an_unknown_chat_is_still_an_error() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 9));
        seed_chat(sandbox, build_local_ts(2020, 3, 17, 12));
        session::init_recent_cutoff().expect("cutoff");
        let before = session::list_session_summaries().expect("before");
        assert!(session::touch_session("spark_chat_missing").is_err());
        assert_eq!(session::list_session_summaries().expect("after"), before);
    });
}
