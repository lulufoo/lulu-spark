//! LW-266: `model_steps.extra` stores `Step::extra`, reads it back, and stays out of the way when empty.

use rusqlite::Connection;
use serde_json::{json, Map, Value};

use crate::agent::session::{self, Step};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    f();
}

fn plain(role: &str, text: &str) -> Step {
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
        clock: Default::default(),
        extra: Default::default(),
    }
}

fn with_extra(mut step: Step, extra: Value) -> Step {
    let Value::Object(map) = extra else {
        panic!("extra must be an object");
    };
    step.extra = map;
    step
}

fn open_db(session_id: &str) -> Connection {
    let path = session::session_file_path(session_id).expect("path");
    Connection::open(path).expect("open sqlite")
}

fn save_steps(session_id: &str, steps: Vec<Step>) -> session::Session {
    let mut loaded = session::load_session(session_id).expect("load");
    loaded.steps = steps;
    session::save_session(&loaded).expect("save");
    session::load_session(session_id).expect("reload")
}

#[test]
fn extra_roundtrips_and_does_not_start_a_new_turn() {
    with_sandbox(|| {
        let id = session::create_session().expect("create").session_id;
        let steps = vec![
            plain("user", "hi"),
            with_extra(plain("assistant", "reply"), json!({ "append_content": "wrap up", "n": [1, { "k": null }] })),
            with_extra(plain("tool", "skipped"), json!({ "skipped": true })),
        ];
        let again = save_steps(&id, steps.clone());
        assert_eq!(again.steps, steps);

        let conn = open_db(&id);
        let turns: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_turns", [], |row| row.get(0))
            .expect("count turns");
        assert_eq!(turns, 1, "extra on non-user steps must not change turn boundaries");
    });
}

#[test]
fn empty_extra_is_null_in_the_row_and_absent_in_json() {
    with_sandbox(|| {
        let id = session::create_session().expect("create").session_id;
        save_steps(&id, vec![plain("user", "hi"), plain("assistant", "a")]);

        let conn = open_db(&id);
        let filled: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_steps WHERE extra IS NOT NULL", [], |row| row.get(0))
            .expect("count");
        assert_eq!(filled, 0);

        let json = serde_json::to_value(plain("assistant", "a")).expect("serialize");
        assert!(json.get("extra").is_none());
        let back: Step = serde_json::from_value(json!({ "role": "assistant", "content": "a" })).expect("deserialize");
        assert_eq!(back.extra, Map::new());
    });
}

#[test]
fn saving_unchanged_steps_with_extra_leaves_earlier_rows_alone() {
    with_sandbox(|| {
        let id = session::create_session().expect("create").session_id;
        let steps = vec![
            plain("user", "q"),
            with_extra(plain("assistant", "a"), json!({ "append_content": "x" })),
        ];
        let mut loaded = save_steps(&id, steps);

        // A sentinel proves the rows are kept: a rewrite would re-stamp `created_at` with `now`.
        open_db(&id)
            .execute("UPDATE messages SET created_at = 111", [])
            .expect("sentinel");
        loaded.steps.push(plain("user", "next"));
        session::save_session(&loaded).expect("save again");

        let conn = open_db(&id);
        let times: Vec<i64> = conn
            .prepare("SELECT created_at FROM messages ORDER BY seq ASC")
            .expect("prepare")
            .query_map([], |row| row.get(0))
            .expect("query")
            .map(|row| row.expect("row"))
            .collect();
        assert_eq!(&times[..2], &[111, 111], "a step with extra must compare equal to its stored row");
    });
}

#[test]
fn old_session_file_gains_the_column_and_loads_with_empty_extra() {
    with_sandbox(|| {
        let id = session::create_session().expect("create").session_id;
        save_steps(&id, vec![plain("user", "old"), plain("assistant", "old reply")]);
        open_db(&id)
            .execute("ALTER TABLE model_steps DROP COLUMN extra", [])
            .expect("drop column to mimic an old database");

        let mut old = session::load_session(&id).expect("old session loads");
        assert_eq!(old.steps.len(), 2);
        assert!(old.steps.iter().all(|step| step.extra.is_empty()));

        old.steps
            .push(with_extra(plain("assistant", "new"), json!({ "skipped": true })));
        session::save_session(&old).expect("save into upgraded db");
        let again = session::load_session(&id).expect("reload");
        assert_eq!(again.steps.len(), 3);
        assert_eq!(again.steps[2].extra["skipped"], json!(true));
    });
}
