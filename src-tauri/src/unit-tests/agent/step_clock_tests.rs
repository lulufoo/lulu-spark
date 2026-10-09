//! LW-180: `model_steps.create_ts / start_ts / end_ts` store, read back, and stay fixed.

use rusqlite::Connection;

use crate::agent::session::{self, Step, StepClock};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    f();
}

fn step_with(role: &str, text: &str, clock: StepClock) -> Step {
    Step {
        role: role.into(),
        content: Some(text.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock,
    }
}

fn clock(create: i64, start: i64, end: i64) -> StepClock {
    StepClock {
        create_ts: Some(create),
        start_ts: Some(start),
        end_ts: Some(end),
    }
}

fn open_db(session_id: &str) -> Connection {
    let path = session::session_file_path(session_id).expect("path");
    Connection::open(path).expect("open")
}

type ClockRow = (Option<i64>, Option<i64>, Option<i64>);

fn read_clocks(conn: &Connection) -> Vec<ClockRow> {
    let mut stmt = conn
        .prepare(
            "SELECT s.create_ts, s.start_ts, s.end_ts FROM model_steps s
             JOIN model_turns t ON t.turn_id = s.turn_id
             ORDER BY t.seq ASC, s.seq ASC",
        )
        .expect("prepare");
    stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("query")
        .map(|row| row.expect("row"))
        .collect()
}

#[test]
fn stamp_helpers_fill_the_marks_the_issue_defines() {
    let point = StepClock::stamp_now();
    assert!(!point.is_empty());
    assert_eq!(point.create_ts, point.start_ts);
    assert_eq!(point.start_ts, point.end_ts);

    let span = StepClock::stamp_span(1_000, 2_500);
    assert_eq!(span.start_ts, Some(1_000));
    assert_eq!(span.end_ts, Some(2_500));
    assert!(span.create_ts.expect("create") >= point.create_ts.expect("point"));

    assert!(StepClock::default().is_empty());
}

#[test]
fn empty_clock_stays_out_of_step_json() {
    let bare = step_with("user", "hi", StepClock::default());
    let json = serde_json::to_value(&bare).expect("json");
    assert!(json.get("clock").is_none());

    let stamped = step_with("user", "hi", clock(1, 2, 3));
    let json = serde_json::to_value(&stamped).expect("json");
    assert_eq!(json["clock"]["start_ts"], 2);
    let back: Step = serde_json::from_value(json).expect("back");
    assert_eq!(back, stamped);
}

#[test]
fn clock_round_trips_through_the_model_steps_columns() {
    with_sandbox(|| {
        let sess = session::create_session().expect("create");
        let mut loaded = session::load_session(&sess.session_id).expect("load");
        loaded.steps.push(step_with("user", "q", clock(10, 10, 10)));
        loaded.steps.push(step_with("assistant", "a", clock(2_100, 1_200, 2_000)));
        session::save_session(&loaded).expect("save");

        let again = session::load_session(&sess.session_id).expect("reload");
        assert_eq!(again.steps[0].clock, clock(10, 10, 10));
        assert_eq!(again.steps[1].clock, clock(2_100, 1_200, 2_000));
        assert_eq!(
            read_clocks(&open_db(&sess.session_id)),
            vec![
                (Some(10), Some(10), Some(10)),
                (Some(2_100), Some(1_200), Some(2_000)),
            ]
        );
    });
}

#[test]
fn unstamped_step_is_stored_as_null_never_zero() {
    with_sandbox(|| {
        let sess = session::create_session().expect("create");
        session::append_step(&sess.session_id, step_with("user", "q", StepClock::default()))
            .expect("append");
        assert_eq!(read_clocks(&open_db(&sess.session_id)), vec![(None, None, None)]);
        let again = session::load_session(&sess.session_id).expect("reload");
        assert!(again.steps[0].clock.is_empty());
    });
}

#[test]
fn saving_again_keeps_earlier_rows_and_their_message_times() {
    with_sandbox(|| {
        let sess = session::create_session().expect("create");
        let mut loaded = session::load_session(&sess.session_id).expect("load");
        loaded.steps.push(step_with("user", "q", clock(10, 10, 10)));
        loaded.steps.push(step_with("assistant", "a", clock(50, 20, 40)));
        session::save_session(&loaded).expect("save");

        // A sentinel proves the row is left alone: a rewrite would re-stamp it with `now`.
        let conn = open_db(&sess.session_id);
        conn.execute("UPDATE messages SET created_at = 111", []).expect("sentinel");
        drop(conn);

        let mut again = session::load_session(&sess.session_id).expect("reload");
        again.steps.push(step_with("user", "next", clock(90, 90, 90)));
        session::save_session(&again).expect("save again");

        let conn = open_db(&sess.session_id);
        let mut stmt = conn
            .prepare("SELECT created_at FROM messages ORDER BY seq ASC")
            .expect("prepare");
        let times: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .expect("query")
            .map(|row| row.expect("row"))
            .collect();
        assert_eq!(times.len(), 3);
        assert_eq!(&times[..2], &[111, 111], "earlier messages must not be rewritten");
        assert_ne!(times[2], 111);
        assert_eq!(
            read_clocks(&conn),
            vec![
                (Some(10), Some(10), Some(10)),
                (Some(50), Some(20), Some(40)),
                (Some(90), Some(90), Some(90)),
            ]
        );
    });
}

#[test]
fn old_session_file_gains_the_columns_and_keeps_old_rows_null() {
    with_sandbox(|| {
        let sess = session::create_session().expect("create");
        session::append_step(&sess.session_id, step_with("user", "old", StepClock::default()))
            .expect("append old");

        let conn = open_db(&sess.session_id);
        for column in ["create_ts", "start_ts", "end_ts"] {
            conn.execute(&format!("ALTER TABLE model_steps DROP COLUMN {column}"), [])
                .expect("drop column");
        }
        drop(conn);

        let mut loaded = session::load_session(&sess.session_id).expect("load old file");
        assert_eq!(loaded.steps.len(), 1);
        assert!(loaded.steps[0].clock.is_empty());

        loaded.steps.push(step_with("assistant", "new", clock(7, 5, 6)));
        session::save_session(&loaded).expect("save new");
        assert_eq!(
            read_clocks(&open_db(&sess.session_id)),
            vec![(None, None, None), (Some(7), Some(5), Some(6))]
        );
    });
}
