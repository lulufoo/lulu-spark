use std::path::PathBuf;

use chrono::{Local, TimeZone};
use rusqlite::{params, Connection};
use serde_json::Value;

use crate::agent::session::{self, Step};
use crate::test_support::TestSandbox;

fn get_catalog_path(sandbox: &TestSandbox) -> PathBuf {
    sandbox
        .cache_dir()
        .join("agent-exec")
        .join("sessions-catalog.sqlite")
}

/// Unix seconds of a local wall-clock time. Fixed past dates keep the tests independent of today.
fn build_local_ts(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
    Local
        .with_ymd_and_hms(y, m, d, h, mi, s)
        .earliest()
        .expect("local time exists")
        .timestamp()
}

fn build_midnight(y: i32, m: u32, d: u32) -> i64 {
    build_local_ts(y, m, d, 0, 0, 0)
}

/// A chat whose catalog `updated_at` is `updated_at`.
fn seed_chat(sandbox: &TestSandbox, updated_at: i64) -> String {
    let id = session::create_session().expect("create").session_id;
    Connection::open(get_catalog_path(sandbox))
        .expect("open catalog")
        .execute(
            "UPDATE sessions SET updated_at = ?2 WHERE session_id = ?1",
            params![id, updated_at],
        )
        .expect("set updated_at");
    id
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
    // The cutoff lives in the process-wide live state; start every test without one.
    session::reset_live_for_tests();
    f(&sandbox);
    session::reset_live_for_tests();
}

fn build_user_step(text: &str) -> Step {
    Step {
        role: "user".into(),
        content: Some(text.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock: Default::default(),
        extra: Default::default(),
    }
}

#[test]
fn no_cutoff_until_two_days_have_chats() {
    run_in_sandbox(|sandbox| {
        assert_eq!(session::init_recent_cutoff().expect("empty"), 0);

        let a = seed_chat(sandbox, build_local_ts(2020, 3, 18, 9, 0, 0));
        let b = seed_chat(sandbox, build_local_ts(2020, 3, 18, 21, 0, 0));
        assert_eq!(session::init_recent_cutoff().expect("one day"), 0);
        let mut ids = list_ids();
        ids.sort();
        let mut want = vec![a, b];
        want.sort();
        assert_eq!(ids, want, "with one active day every chat is listed");
    });
}

#[test]
fn cutoff_is_local_midnight_of_the_older_of_the_two_latest_days() {
    run_in_sandbox(|sandbox| {
        let today = seed_chat(sandbox, build_local_ts(2020, 3, 18, 10, 0, 0));
        let yesterday = seed_chat(sandbox, build_local_ts(2020, 3, 17, 15, 30, 0));
        let old = seed_chat(sandbox, build_local_ts(2020, 3, 10, 12, 0, 0));

        let cutoff = session::init_recent_cutoff().expect("cutoff");
        assert_eq!(cutoff, build_midnight(2020, 3, 17));
        let ids = list_ids();
        assert!(ids.contains(&today));
        assert!(ids.contains(&yesterday));
        assert!(!ids.contains(&old), "a chat before the cutoff is not listed");
    });
}

#[test]
fn the_two_days_need_not_be_adjacent() {
    run_in_sandbox(|sandbox| {
        let latest = seed_chat(sandbox, build_local_ts(2020, 3, 18, 8, 0, 0));
        let earlier = seed_chat(sandbox, build_local_ts(2020, 3, 13, 20, 0, 0));
        let older = seed_chat(sandbox, build_local_ts(2020, 3, 10, 20, 0, 0));

        let cutoff = session::init_recent_cutoff().expect("cutoff");
        assert_eq!(cutoff, build_midnight(2020, 3, 13));
        let ids = list_ids();
        assert!(ids.contains(&latest) && ids.contains(&earlier));
        assert!(!ids.contains(&older));
    });
}

#[test]
fn several_chats_on_one_day_count_as_one_day() {
    run_in_sandbox(|sandbox| {
        for hour in [9, 10, 11] {
            seed_chat(sandbox, build_local_ts(2020, 3, 18, hour, 0, 0));
        }
        let other = seed_chat(sandbox, build_local_ts(2020, 3, 15, 12, 0, 0));
        seed_chat(sandbox, build_local_ts(2020, 3, 1, 12, 0, 0));

        let cutoff = session::init_recent_cutoff().expect("cutoff");
        assert_eq!(cutoff, build_midnight(2020, 3, 15));
        assert_eq!(list_ids().len(), 4);
        assert!(list_ids().contains(&other));
    });
}

#[test]
fn the_cutoff_boundary_is_inclusive() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 12, 0, 0));
        let at_midnight = seed_chat(sandbox, build_midnight(2020, 3, 17));
        let just_before = seed_chat(sandbox, build_midnight(2020, 3, 17) - 1);

        let cutoff = session::init_recent_cutoff().expect("cutoff");
        assert_eq!(cutoff, build_midnight(2020, 3, 17));
        let ids = list_ids();
        assert!(ids.contains(&at_midnight), "exactly at the cutoff is listed");
        assert!(!ids.contains(&just_before), "one second before the cutoff is not");
    });
}

#[test]
fn a_chat_without_a_time_is_not_a_day() {
    run_in_sandbox(|sandbox| {
        let no_time = seed_chat(sandbox, 0);
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 12, 0, 0));
        assert_eq!(session::init_recent_cutoff().expect("one real day"), 0);
        assert!(list_ids().contains(&no_time), "no cutoff lists it");

        seed_chat(sandbox, build_local_ts(2020, 3, 17, 12, 0, 0));
        let cutoff = session::init_recent_cutoff().expect("two real days");
        assert_eq!(cutoff, build_midnight(2020, 3, 17));
        assert!(!list_ids().contains(&no_time), "a cutoff leaves it out");
    });
}

#[test]
fn the_list_is_newest_first() {
    run_in_sandbox(|sandbox| {
        let a = seed_chat(sandbox, build_local_ts(2020, 3, 17, 9, 0, 0));
        let b = seed_chat(sandbox, build_local_ts(2020, 3, 18, 9, 0, 0));
        let c = seed_chat(sandbox, build_local_ts(2020, 3, 17, 18, 0, 0));
        session::init_recent_cutoff().expect("cutoff");
        assert_eq!(list_ids(), vec![b, c, a]);
    });
}

#[test]
fn the_cutoff_stays_put_after_launch() {
    run_in_sandbox(|sandbox| {
        let kept = seed_chat(sandbox, build_local_ts(2020, 3, 13, 12, 0, 0));
        seed_chat(sandbox, build_local_ts(2020, 3, 17, 12, 0, 0));
        let cutoff = session::init_recent_cutoff().expect("cutoff");
        assert_eq!(cutoff, build_midnight(2020, 3, 13));

        // A chat created now makes a new active day. The older day must stay listed.
        let fresh = session::create_session().expect("create").session_id;
        let ids = list_ids();
        assert!(ids.contains(&fresh), "a chat created after launch is listed");
        assert!(ids.contains(&kept), "the older day is not pushed out mid-run");
    });
}

#[test]
fn touching_a_chat_sets_its_time_and_keeps_the_title() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 12, 0, 0));
        seed_chat(sandbox, build_local_ts(2020, 3, 17, 12, 0, 0));
        let old = session::create_session().expect("create").session_id;
        session::append_step(&old, build_user_step("Question from March 1")).expect("append");
        let old_ts = build_local_ts(2020, 3, 1, 12, 0, 0);
        let conn = Connection::open(get_catalog_path(sandbox)).expect("open catalog");
        conn.execute(
            "UPDATE sessions SET updated_at = ?2 WHERE session_id = ?1",
            params![old, old_ts],
        )
        .expect("age it");
        let db = session::session_file_path(&old).expect("path");
        Connection::open(&db)
            .expect("open session")
            .execute("UPDATE meta SET updated_at = ?1", params![old_ts])
            .expect("age meta");

        session::init_recent_cutoff().expect("cutoff");
        assert!(!list_ids().contains(&old), "before touching, it is outside the list");

        session::touch_session(&old).expect("touch");

        assert!(list_ids().contains(&old), "after touching it is in the list");
        let (title, catalog_ts): (String, i64) = conn
            .query_row(
                "SELECT title, updated_at FROM sessions WHERE session_id = ?1",
                params![old],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("catalog row");
        assert_eq!(title, "Question from March 1", "the title is untouched");
        assert!(catalog_ts > old_ts);
        let meta_ts: i64 = Connection::open(&db)
            .expect("open session")
            .query_row("SELECT updated_at FROM meta LIMIT 1", [], |row| row.get(0))
            .expect("meta row");
        assert_eq!(meta_ts, catalog_ts, "the session database and the catalog agree");
        let steps = session::load_session(&old).expect("load").steps;
        assert_eq!(steps.len(), 1, "the turns are untouched");
    });
}

#[test]
fn touching_an_unknown_chat_is_an_error_and_writes_nothing() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 12, 0, 0));
        let before: Vec<Value> = session::list_session_summaries().expect("before");
        assert!(session::touch_session("spark_chat_missing").is_err());
        assert_eq!(session::list_session_summaries().expect("after"), before);
    });
}

#[test]
fn selecting_a_chat_through_the_command_brings_it_into_the_list() {
    run_in_sandbox(|sandbox| {
        seed_chat(sandbox, build_local_ts(2020, 3, 18, 12, 0, 0));
        seed_chat(sandbox, build_local_ts(2020, 3, 17, 12, 0, 0));
        let old = seed_chat(sandbox, build_local_ts(2020, 3, 1, 12, 0, 0));
        session::init_recent_cutoff().expect("cutoff");
        assert!(!list_ids().contains(&old));

        crate::commands::ai_assistant::select_chat_session_json(&old).expect("select");

        let listed = crate::commands::ai_assistant::list_chat_sessions_json().expect("list");
        let ids: Vec<&str> = listed["sessions"]
            .as_array()
            .expect("sessions")
            .iter()
            .map(|s| s["session_id"].as_str().expect("id"))
            .collect();
        assert_eq!(ids.first(), Some(&old.as_str()), "the opened chat is now the newest");
    });
}
