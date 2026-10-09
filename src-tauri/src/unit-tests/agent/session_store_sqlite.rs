use rusqlite::Connection;
use serde_json::json;

use crate::agent::session::{self, Step};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    f(&sandbox);
}

fn user_step(text: &str) -> Step {
    Step {
        role: "user".into(),
        content: Some(text.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    }
}

fn assistant_step(text: &str) -> Step {
    Step {
        role: "assistant".into(),
        content: Some(text.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    }
}

#[test]
fn create_session_writes_sqlite_not_json() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("create");
        let path = session::session_file_path(&sess.session_id).expect("path");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("sqlite"));
        assert!(path.is_file());
        assert!(!path.with_extension("json").is_file());
        let catalog = sandbox
            .cache_dir()
            .join("agent")
            .join("sessions-catalog.sqlite");
        assert!(catalog.is_file());
        sandbox
            .assert_not_prod_path(&path)
            .expect("session sqlite must stay in sandbox");
        sandbox
            .assert_not_prod_path(&catalog)
            .expect("catalog must stay in sandbox");
    });
}

#[test]
fn save_roundtrip_keeps_user_assistant_and_tool_turns() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("create");
        let mut loaded = session::load_session(&sess.session_id).expect("load");
        loaded.steps.push(user_step("hello"));
        loaded.steps.push(Step {
            role: "assistant".into(),
            content: None,
            tool_call_id: None,
            tool_calls: Some(json!([{
                "id": "call_1",
                "type": "function",
                "function": { "name": "read", "arguments": "{}" }
            }])),
            name: None,
        });
        loaded.steps.push(Step {
            role: "tool".into(),
            content: Some("ok".into()),
            tool_call_id: Some("call_1".into()),
            tool_calls: None,
            name: Some("read".into()),
        });
        loaded.steps.push(assistant_step("done"));
        session::save_session(&loaded).expect("save");

        let again = session::load_session(&sess.session_id).expect("reload");
        assert_eq!(again.steps.len(), 4);
        assert_eq!(again.steps[0].content.as_deref(), Some("hello"));
        assert!(again.steps[1].tool_calls.is_some());
        assert_eq!(again.steps[2].role, "tool");
        assert_eq!(again.steps[2].name.as_deref(), Some("read"));
        assert_eq!(again.steps[3].content.as_deref(), Some("done"));

        let ui = session::load_turns_value(&sess.session_id);
        let items = ui.as_array().expect("ui array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["role"], "user");
        assert_eq!(items[1]["role"], "assistant");
        assert_eq!(items[1]["content"], "done");
    });
}

#[test]
fn catalog_lists_title_and_delete_removes_row() {
    with_sandbox(|_| {
        let first = session::create_session().expect("first");
        session::append_step(&first.session_id, user_step("first title here")).expect("append");
        let second = session::create_session().expect("second");
        session::append_step(&second.session_id, user_step("second")).expect("append");

        let listed = session::list_session_summaries().expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0]["session_id"], second.session_id);
        assert_eq!(listed[0]["title"], "second");
        assert_eq!(listed[1]["title"], "first title here");

        session::delete_session(&first.session_id).expect("delete");
        let after = session::list_session_summaries().expect("list after");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0]["session_id"], second.session_id);
        assert!(!session::session_file_path(&first.session_id)
            .expect("path")
            .is_file());
    });
}

#[test]
fn session_sqlite_keeps_summaries_table_empty() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("create");
        session::append_step(&sess.session_id, user_step("x")).expect("append");
        let path = session::session_file_path(&sess.session_id).expect("path");
        let conn = Connection::open(&path).expect("open");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM summaries", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 0);
        let messages: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .expect("messages");
        assert_eq!(messages, 1);
        let steps: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_turns", [], |row| row.get(0))
            .expect("turns");
        assert_eq!(steps, 1);
    });
}

#[test]
fn save_appends_new_turns_without_deleting_detached_steps() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("create");
        session::append_step(&sess.session_id, user_step("first")).expect("append user");
        let path = session::session_file_path(&sess.session_id).expect("path");
        let conn = Connection::open(&path).expect("open");
        conn.execute(
            "INSERT INTO model_steps (step_id, turn_id, seq, kind, content)
             VALUES ('detached_step', 'detached_turn', 1, 'tool_result', 'archived raw step')",
            [],
        )
        .expect("insert detached step");
        drop(conn);

        session::append_step(&sess.session_id, assistant_step("reply")).expect("append assistant");

        let conn = Connection::open(&path).expect("reopen");
        let retained: String = conn
            .query_row(
                "SELECT content FROM model_steps WHERE step_id = 'detached_step'",
                [],
                |row| row.get(0),
            )
            .expect("detached step must remain");
        assert_eq!(retained, "archived raw step");
    });
}

#[test]
fn save_truncates_only_the_removed_turn_suffix() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("create");
        session::append_step(&sess.session_id, user_step("first")).expect("append user");
        session::append_step(&sess.session_id, assistant_step("reply")).expect("append assistant");

        let mut loaded = session::load_session(&sess.session_id).expect("load");
        loaded.steps.truncate(1);
        session::save_session(&loaded).expect("save truncation");

        let reloaded = session::load_session(&sess.session_id).expect("reload");
        assert_eq!(reloaded.steps.len(), 1);
        assert_eq!(reloaded.steps[0].role, "user");
        assert_eq!(session::load_turns_value(&sess.session_id), json!([
            {"role": "user", "content": "first"}
        ]));
    });
}

#[test]
fn save_writes_only_changed_staged_entries() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("create");
        let mut loaded = session::load_session(&sess.session_id).expect("load");
        loaded
            .register_staged("/sandbox/plan.md", Some("Plan"), None)
            .expect("stage");
        session::save_session(&loaded).expect("save stage");

        let path = session::session_file_path(&sess.session_id).expect("path");
        let conn = Connection::open(&path).expect("open");
        conn.execute_batch(
            "CREATE TABLE stage_audit (updates INTEGER NOT NULL);
             INSERT INTO stage_audit VALUES (0);
             CREATE TRIGGER count_staged_updates AFTER UPDATE ON staged
             BEGIN
               UPDATE stage_audit SET updates = updates + 1;
             END;",
        )
        .expect("install audit trigger");
        drop(conn);

        session::save_session(&loaded).expect("save unchanged stage");
        let conn = Connection::open(&path).expect("reopen");
        let updates: i64 = conn
            .query_row("SELECT updates FROM stage_audit", [], |row| row.get(0))
            .expect("read unchanged update count");
        assert_eq!(updates, 0);
        drop(conn);

        loaded.staged[0].title = "Renamed plan".into();
        session::save_session(&loaded).expect("save changed stage");
        let conn = Connection::open(&path).expect("reopen after update");
        let updates: i64 = conn
            .query_row("SELECT updates FROM stage_audit", [], |row| row.get(0))
            .expect("read changed update count");
        assert_eq!(updates, 1);
    });
}

#[test]
fn catalog_stamps_create_llm_and_keeps_empty_unset() {
    with_sandbox(|_| {
        let first = session::create_session().expect("first");
        assert_eq!(first.llm.as_deref(), Some("host"));
        let listed = session::list_session_summaries().expect("list");
        assert_eq!(listed[0]["llm"], "host");

        let mut settings = crate::config::settings::load().expect("load settings");
        settings.assistant_engine = "claude".into();
        crate::config::settings::save(&settings).expect("save settings");
        let second = session::create_session().expect("second");
        assert_eq!(second.llm.as_deref(), Some("claude"));
        let again = session::load_session(&first.session_id).expect("reload first");
        assert_eq!(again.llm.as_deref(), Some("host"));

        let listed = session::list_session_summaries().expect("list two");
        let claude = listed
            .iter()
            .find(|row| row["session_id"] == second.session_id)
            .expect("claude row");
        let host = listed
            .iter()
            .find(|row| row["session_id"] == first.session_id)
            .expect("host row");
        assert_eq!(claude["llm"], "claude");
        assert_eq!(host["llm"], "host");

        let bare = session::Session {
            session_id: "spark_chat_legacy_llm".into(),
            steps: Vec::new(),
            staged: Vec::new(),
            llm: None,
        };
        session::save_session(&bare).expect("save bare");
        let loaded = session::load_session(&bare.session_id).expect("load bare");
        assert!(loaded.llm.is_none());
        let listed = session::list_session_summaries().expect("list bare");
        let row = listed
            .iter()
            .find(|item| item["session_id"] == bare.session_id)
            .expect("bare row");
        assert!(row.get("llm").is_none());
    });
}
