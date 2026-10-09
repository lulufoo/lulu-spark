//! LW-169: `reasoning_content` and the response-level columns survive save/load.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::agent::session::{self, Step};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    f(&sandbox);
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
    }
}

fn with_response(mut step: Step, finish: &str) -> Step {
    step.finish_reason = Some(finish.into());
    step.model = Some("glm-test".into());
    step.usage = Some(json!({ "prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5 }));
    step
}

fn with_thinking(mut step: Step, thinking: &str) -> Step {
    step.reasoning_content = Some(thinking.into());
    step
}

fn tool_call_step(call_count: usize) -> Step {
    let calls: Vec<Value> = (0..call_count)
        .map(|i| {
            json!({
                "id": format!("call_{i}"),
                "type": "function",
                "function": { "name": "read", "arguments": "{}" }
            })
        })
        .collect();
    Step {
        role: "assistant".into(),
        content: None,
        tool_call_id: None,
        tool_calls: Some(Value::Array(calls)),
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock: Default::default(),
    }
}

fn open_db(session_id: &str) -> Connection {
    let path = session::session_file_path(session_id).expect("path");
    Connection::open(path).expect("open sqlite")
}

fn step_ids(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT step_id FROM model_steps ORDER BY rowid")
        .expect("prepare");
    stmt.query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .map(|row| row.expect("row"))
        .collect()
}

fn save_steps(session_id: &str, steps: Vec<Step>) -> session::Session {
    let mut loaded = session::load_session(session_id).expect("load");
    loaded.steps = steps;
    session::save_session(&loaded).expect("save");
    session::load_session(session_id).expect("reload")
}

#[test]
fn reasoning_content_and_response_fields_roundtrip_exactly() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        let steps = vec![
            plain("user", "hi"),
            with_thinking(with_response(tool_call_step(1), "tool_calls"), "先查一下"),
            Step {
                tool_call_id: Some("call_0".into()),
                name: Some("read".into()),
                ..plain("tool", "ok")
            },
            with_thinking(with_response(plain("assistant", "done"), "sensitive"), "再想想"),
        ];
        let again = save_steps(&id, steps.clone());
        assert_eq!(again.steps, steps);
        assert_eq!(again.steps[1].reasoning_content.as_deref(), Some("先查一下"));
        assert_eq!(again.steps[3].finish_reason.as_deref(), Some("sensitive"));
        assert_eq!(again.steps[3].reasoning_content.as_deref(), Some("再想想"));
    });
}

#[test]
fn several_tool_calls_take_one_row_carrying_the_response_fields() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        save_steps(
            &id,
            vec![
                plain("user", "go"),
                with_thinking(with_response(tool_call_step(3), "tool_calls"), "三个一起读"),
            ],
        );
        let conn = open_db(&id);
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_steps WHERE kind = 'tool_call'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(rows, 1);
        let (finish, model, usage, thinking): (String, String, String, String) = conn
            .query_row(
                "SELECT finish_reason, model, usage, reasoning_content
                 FROM model_steps WHERE kind = 'tool_call'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("fields");
        assert_eq!(thinking, "三个一起读");
        assert_eq!(finish, "tool_calls");
        assert_eq!(model, "glm-test");
        assert!(usage.contains("total_tokens"));
    });
}

#[test]
fn empty_response_fallback_step_keeps_the_response_fields() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        // The fallback text is synthetic; the fields and thinking belong to the model's response.
        let steps = vec![
            plain("user", "think hard"),
            with_thinking(
                with_response(plain("assistant", "模型响应为空，未执行任何写入。"), "sensitive"),
                "只想不说",
            ),
        ];
        let again = save_steps(&id, steps.clone());
        assert_eq!(again.steps, steps);
        let last = again.steps.last().expect("last");
        assert_eq!(last.finish_reason.as_deref(), Some("sensitive"));
        assert_eq!(last.reasoning_content.as_deref(), Some("只想不说"));
    });
}

#[test]
fn saving_unchanged_steps_twice_does_not_rewrite_rows() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        let steps = vec![
            plain("user", "hi"),
            with_thinking(with_response(plain("assistant", "ok"), "stop"), "想"),
        ];
        // Save the in-memory steps (with every field set) twice. If any field is not
        // restored on load, `sync` sees stored != current and rewrites the rows.
        let mut live = session::load_session(&id).expect("load");
        live.steps = steps;
        session::save_session(&live).expect("save");
        let before = step_ids(&open_db(&id));
        session::save_session(&live).expect("save again");
        let after = step_ids(&open_db(&id));
        assert_eq!(before.len(), 2);
        assert_eq!(before, after, "sync must keep identical rows untouched");
    });
}

#[test]
fn reasoning_content_is_a_ui_field_and_stays_out_of_content() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        save_steps(
            &id,
            vec![
                plain("user", "hi"),
                with_thinking(with_response(plain("assistant", "done"), "stop"), "secret thoughts"),
            ],
        );
        session::store_turn_thinking_ms(&id, 1400).expect("thinking_ms");
        let ui = session::load_turns_value(&id);
        let items = ui.as_array().expect("ui array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[1]["content"], "done");
        assert_eq!(items[1]["thinking"], "secret thoughts");
        assert_eq!(items[1]["thinking_ms"], 1400);
        assert_ne!(items[1]["content"], "secret thoughts");
    });
}

#[test]
fn one_turn_joins_thinking_pieces_on_the_final_assistant_ui_message() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        save_steps(
            &id,
            vec![
                plain("user", "hi"),
                with_thinking(with_response(tool_call_step(1), "tool_calls"), "先看"),
                Step {
                    role: "tool".into(),
                    content: Some("ok".into()),
                    tool_call_id: Some("call_0".into()),
                    tool_calls: None,
                    name: Some("read".into()),
                    finish_reason: None,
                    model: None,
                    usage: None,
                    reasoning_content: None,
                    clock: Default::default(),
                },
                with_thinking(with_response(plain("assistant", "done"), "stop"), "再想"),
            ],
        );
        let ui = session::load_turns_value(&id);
        let items = ui.as_array().expect("ui array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[1]["content"], "done");
        assert_eq!(items[1]["thinking"], "先看\n\n再想");
    });
}

#[test]
fn old_session_without_the_new_columns_loads_and_can_be_saved_again() {
    with_sandbox(|_| {
        let id = session::create_session().expect("create").session_id;
        save_steps(&id, vec![plain("user", "old"), plain("assistant", "old reply")]);
        {
            let conn = open_db(&id);
            for column in ["finish_reason", "model", "usage", "reasoning_content"] {
                conn.execute(&format!("ALTER TABLE model_steps DROP COLUMN {column}"), [])
                    .expect("drop column to mimic an old database");
            }
        }
        let mut old = session::load_session(&id).expect("old session loads");
        assert_eq!(old.steps.len(), 2);
        assert!(old.steps.iter().all(|s| s.finish_reason.is_none() && s.usage.is_none() && s.reasoning_content.is_none()));

        old.steps.push(with_response(plain("assistant", "new"), "stop"));
        session::save_session(&old).expect("save into upgraded db");
        let again = session::load_session(&id).expect("reload");
        assert_eq!(again.steps.len(), 3);
        assert_eq!(again.steps[2].finish_reason.as_deref(), Some("stop"));
    });
}
