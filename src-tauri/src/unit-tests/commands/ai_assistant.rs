//! Host command surface tests for open_ai_assistant / Present / agent_chat_turn.

use std::fs;

use serde_json::json;

use crate::commands::ai_assistant::{
    agent_chat_turn_json, cancel_ai_assistant_turn_json, create_chat_session_json,
    defensive_unbound_json, ensure_ai_assistant_session_json, execute_binding_json,
    get_ai_assistant_binding_json, list_chat_sessions_json, open_ai_assistant_json,
    present_ai_assistant_json, query_binding_json, record_ai_assistant_timing,
    reset_binding_json, select_chat_session_json, set_binding_json, shell_close_json,
    AI_ASSISTANT_WINDOW_LABEL, EVENT_BINDING_CHANGED,
};
use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::services::agent::r#loop;
use crate::services::agent::session::{self, Turn};
use crate::services::todo_task;
use crate::test_support::TestSandbox;

fn is_session_when_label(title: &str) -> bool {
    let bytes = title.as_bytes();
    bytes.len() == 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b' '
        && bytes[13] == b':'
        && bytes
            .iter()
            .all(|b| b.is_ascii_digit() || *b == b'-' || *b == b' ' || *b == b':')
}

fn with_cmd_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    crate::services::mcp_server_registry::clear_for_tests();
    crate::services::mcp_server_registry::seed_defaults();
    f();
}

fn create_plan(title: &str) -> String {
    let created = todo_task::create_master_with_subs(title, Some(&["子A"]));
    created["master_task_id"].as_str().unwrap().to_string()
}

#[test]
fn open_ai_assistant_json_returns_window_label_constant() {
    with_cmd_sandbox(|| {
        let id = create_plan("命令打开");
        let v = open_ai_assistant_json(&id).expect("open");
        assert_eq!(v["window_label"], AI_ASSISTANT_WINDOW_LABEL);
        assert_eq!(AI_ASSISTANT_WINDOW_LABEL, "ai-assistant");
        assert!(v.get("bound_master_task_id").is_none());
        assert!(!v["session_id"].as_str().unwrap_or("").is_empty());
    });
}

#[test]
fn ensure_ai_assistant_session_provisions_session_without_present_set() {
    with_cmd_sandbox(|| {
        let _id = create_plan("ensure会话");
        assert_eq!(query_binding_json()["state"], "unbound");
        let v = ensure_ai_assistant_session_json().expect("ensure");
        assert!(!v["session_id"].as_str().unwrap_or("").is_empty());
        assert!(v.get("bound_master_task_id").is_none());
        // T3 / L09-AR: ensure payload must not carry surface Present (consumers must not openEntry).
        assert!(
            v.get("surface").is_none() || v["surface"] != "Present",
            "ensure must not emit surface Present: {v}"
        );
        assert!(v.get("entry_id").is_none(), "ensure must not carry Present entry_id: {v}");
        // ensure must not Set Binding Contract (Present≠Set; ensure≠Set).
        assert_eq!(query_binding_json()["state"], "unbound");
        assert_eq!(EVENT_BINDING_CHANGED, "ai-assistant:binding-changed");
        let pulled = get_ai_assistant_binding_json();
        assert_eq!(pulled["session_id"], v["session_id"]);
        assert!(pulled.get("bound_master_task_id").is_none());
    });
}

#[test]
fn agent_chat_turn_json_without_binding_is_business() {
    with_cmd_sandbox(|| {
        let a = create_plan("绑A");
        let open = open_ai_assistant_json(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        // No Binding Contract Set → unbound business terminal (master arg ignored).
        let result = agent_chat_turn_json(sid, "hi", Some(&a)).unwrap();
        assert_eq!(result.body["terminal"], "business");
        assert!(result.emit_turn_completed.is_some());
    });
}

#[test]
fn commands_module_exports_match_acl_names() {
    // Smoke: symbols exist for generate_handler registration.
    let _ = open_ai_assistant_json;
    let _ = agent_chat_turn_json;
    let _ = present_ai_assistant_json;
    let _ = json!({ "ok": true });
    let mut s = settings::load().unwrap();
    settings::apply_config_payload(
        &mut s,
        &json!({ "llm": { "base_url": "http://127.0.0.1:9", "model": "m", "platform": "kimi" } }),
    )
    .expect("apply");
    let _ = settings::save(&s);
    let _ = secrets::set_secret(KEY_LLM_API_KEY, "k");
}

#[test]
fn present_ai_assistant_json_is_shell_only_not_bound() {
    with_cmd_sandbox(|| {
        let v = present_ai_assistant_json().expect("Present");
        assert_eq!(v["surface"], "Present");
        assert_eq!(v["window_label"], AI_ASSISTANT_WINDOW_LABEL);
        // T3 / L09-AR: Present payload extends entry_id for shell openEntry.
        assert_eq!(v["entry_id"], "ai-assistant");
        assert_eq!(query_binding_json()["state"], "unbound");
        // Present must not write business binding primary key into contract query.
        assert!(query_binding_json().get("bound_master_task_id").is_none());
        assert!(query_binding_json().get("master_task_id").is_none());
    });
}

/// t3 / L06-T: Present must not write legacy bound_master_task_id (open_ai_assistant_core side effect).
#[test]
fn present_ai_assistant_does_not_write_bound_master_task_id() {
    with_cmd_sandbox(|| {
        let _ = present_ai_assistant_json().expect("Present");
        let binding = get_ai_assistant_binding_json();
        assert_eq!(
            binding["bound_master_task_id"].as_str().unwrap_or(""),
            "",
            "Present must not write bound_master_task_id"
        );
        assert_eq!(query_binding_json()["state"], "unbound");
        let exec = execute_binding_json();
        assert_eq!(exec["ok"], false);
        assert_eq!(exec["code"], "rejected_unbound");
    });
}

#[test]
fn open_ai_assistant_present_path_does_not_imply_set() {
    with_cmd_sandbox(|| {
        let id = create_plan("开窗非Set");
        let _ = open_ai_assistant_json(&id).expect("open/Present path");
        assert_eq!(
            r#loop::binding_state(),
            "unbound",
            "open_ai_assistant must not imply Binding Contract Set/bound"
        );
        assert_eq!(query_binding_json()["state"], "unbound");
    });
}

/// J1 command-surface fixture: generic Binding (empty callbacks) drives Set→execute→Reset→reject.
/// No business-page / 角位 click driver.
#[test]
fn j1_command_fixture_set_execute_reset_reject_via_json() {
    with_cmd_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let binding = json!({ "key": SEEDED_BUSINESS_KEY });
        let set = set_binding_json(binding);
        assert_eq!(set["ok"], true);
        assert_eq!(set["state"], "bound");
        assert!(r#loop::loaded_mcp_server().is_some());

        let exec = execute_binding_json();
        assert_eq!(exec["ok"], true);
        // Host-filled prompt from loaded MCP capability description (not client legacy payload).
        assert!(exec["applied_prompt"].as_str().unwrap_or("").len() > 0);

        let reset = reset_binding_json();
        assert_eq!(reset["ok"], true);
        assert_eq!(reset["state"], "unbound");

        let rejected = execute_binding_json();
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["code"], "rejected_unbound");
        assert_eq!(query_binding_json()["state"], "unbound");
    });
}

#[test]
fn j1_command_illegal_set_emits_set_invalid_keeps_unbound() {
    with_cmd_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        // Legacy tools/prompt/callbacks payload must fail at public boundary.
        let bad = set_binding_json(json!({
            "tools": [],
            "prompt": "",
            "callbacks": {}
        }));
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["code"], "set_invalid");
        assert_eq!(bad["state"], "unbound");
        let events = r#loop::drain_lifecycle_events();
        assert!(events.iter().all(|e| e.event != "onBound"), "{events:?}");
        assert!(
            events
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("set_invalid")),
            "J1-(2) command path must observe onError(set_invalid): {events:?}"
        );
    });
}

#[test]
fn j1_present_not_bound_execute_rejects_without_set() {
    with_cmd_sandbox(|| {
        let presented = present_ai_assistant_json().expect("Present");
        assert_eq!(presented["surface"], "Present");
        assert_eq!(query_binding_json()["state"], "unbound");
        let exec = execute_binding_json();
        assert_eq!(exec["ok"], false);
        assert_eq!(exec["code"], "rejected_unbound");
    });
}

/// T5: defensive cut Host entry must emit binding-changed (not core-only bypass).
#[test]
fn t5_defensive_unbound_json_returns_unbound_and_shares_binding_changed_event() {
    with_cmd_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let binding = json!({ "key": SEEDED_BUSINESS_KEY });
        assert_eq!(set_binding_json(binding)["ok"], true);
        assert_eq!(query_binding_json()["state"], "bound");

        let cut = defensive_unbound_json();
        assert_eq!(cut["ok"], true);
        assert_eq!(cut["state"], "unbound");
        assert_eq!(query_binding_json()["state"], "unbound");
        // Same event name the Set/Reset command paths emit to the shell.
        assert_eq!(EVENT_BINDING_CHANGED, "ai-assistant:binding-changed");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == EVENT_BINDING_CHANGED && e.state == "unbound"
            }),
            "defensive_unbound_json must record shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t5_execute_json_exposes_distinguishable_reject_codes() {
    with_cmd_sandbox(|| {
        let unbound = execute_binding_json();
        assert_eq!(unbound["ok"], false);
        assert_eq!(unbound["code"], "rejected_unbound");

        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let binding = json!({ "key": SEEDED_BUSINESS_KEY });
        assert_eq!(set_binding_json(binding.clone())["ok"], true);
        // Mid-execute Reset → reset_cancelled (Host signal).
        r#loop::clear_lifecycle_events_for_tests();
        let cancel = r#loop::execute_binding_during(|| {
            let _ = reset_binding_json();
        })
        .expect_err("cancel");
        assert_eq!(cancel.as_code(), "reset_cancelled");

        assert_eq!(set_binding_json(binding)["ok"], true);
        let stale = r#loop::execute_binding_during(|| {
            let _ = set_binding_json(json!({ "key": SEEDED_BUSINESS_KEY }));
        })
        .expect_err("stale");
        assert_eq!(stale.as_code(), "rejected_stale_generation");
    });
}

#[test]
fn cancel_ai_assistant_turn_marks_live_chat_cancelled_without_resetting_binding() {
    with_cmd_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;

        assert_eq!(
            set_binding_json(json!({ "key": SEEDED_BUSINESS_KEY }))["ok"],
            true
        );
        ensure_ai_assistant_session_json().expect("live session for cancel");
        r#loop::set_busy_for_tests(true);

        let result = cancel_ai_assistant_turn_json();

        assert_eq!(result["ok"], true);
        assert_eq!(r#loop::is_chat_cancelled_for_tests(), true);
        assert_eq!(query_binding_json()["state"], "bound");
    });
}

/// SK-3 / T5: binding pull exposes live-session turns (read-only hydrate).
#[test]
fn sk3_t5_binding_returns_live_session_turns() {
    with_cmd_sandbox(|| {
        let ensured = ensure_ai_assistant_session_json().expect("ensure");
        let sid = ensured["session_id"].as_str().unwrap().to_string();
        session::append_turn(
            &sid,
            Turn {
                role: "user".into(),
                content: Some("hello hydrate".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append user");
        session::append_turn(
            &sid,
            Turn {
                role: "assistant".into(),
                content: Some("world hydrate".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append assistant");

        let pulled = get_ai_assistant_binding_json();
        assert_eq!(pulled["session_id"], sid);
        let turns = pulled["turns"].as_array().expect("turns array");
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0]["role"], "user");
        assert_eq!(turns[0]["content"], "hello hydrate");
        assert_eq!(turns[1]["role"], "assistant");
        assert_eq!(turns[1]["content"], "world hydrate");
        // Disk read must reuse session::load_session (same content as file).
        let disk = session::load_session(&sid).expect("disk");
        assert_eq!(disk.turns.len(), 2);
        assert_eq!(disk.turns[0].content.as_deref(), Some("hello hydrate"));
    });
}

#[test]
fn sk3_t5_binding_turns_empty_without_live_or_after_reset() {
    with_cmd_sandbox(|| {
        let empty = get_ai_assistant_binding_json();
        assert_eq!(empty["session_id"], "");
        assert_eq!(empty["turns"].as_array().expect("turns").len(), 0);

        let ensured = ensure_ai_assistant_session_json().expect("ensure");
        let sid = ensured["session_id"].as_str().unwrap().to_string();
        session::append_turn(
            &sid,
            Turn {
                role: "user".into(),
                content: Some("will cut".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append");
        assert_eq!(
            get_ai_assistant_binding_json()["turns"]
                .as_array()
                .expect("turns")
                .len(),
            1
        );

        // Set then Reset cuts live; turns must be empty (not executable old session).
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let binding = json!({ "key": SEEDED_BUSINESS_KEY });
        assert_eq!(set_binding_json(binding)["ok"], true);
        // Successful Set itself clears live session id.
        let after_set = get_ai_assistant_binding_json();
        assert_eq!(after_set["session_id"], "");
        assert_eq!(after_set["turns"].as_array().expect("turns").len(), 0);

        let ensured2 = ensure_ai_assistant_session_json().expect("ensure2");
        let sid2 = ensured2["session_id"].as_str().unwrap().to_string();
        session::append_turn(
            &sid2,
            Turn {
                role: "user".into(),
                content: Some("live2".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append2");
        assert_eq!(reset_binding_json()["ok"], true);
        let after_reset = get_ai_assistant_binding_json();
        assert_eq!(after_reset["session_id"], "");
        assert_eq!(after_reset["turns"].as_array().expect("turns").len(), 0);
        // Old session id is not live → chat reject (不可执行旧会话).
        let rejected = agent_chat_turn_json(&sid2, "hi", None).unwrap();
        assert_eq!(rejected.body["code"], "rejected_not_live_session");
    });
}

#[test]
fn sk3_t5_shell_close_preserves_live_turns_for_reopen_hydrate() {
    with_cmd_sandbox(|| {
        let ensured = ensure_ai_assistant_session_json().expect("ensure");
        let sid = ensured["session_id"].as_str().unwrap().to_string();
        session::append_turn(
            &sid,
            Turn {
                role: "user".into(),
                content: Some("keep across close".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append");

        let closed = shell_close_json();
        assert_eq!(closed["ok"], true);
        // 关壳 ≠ Reset: live session + turns remain for remount hydrate.
        assert_eq!(query_binding_json()["state"], "unbound");
        let pulled = get_ai_assistant_binding_json();
        assert_eq!(pulled["session_id"], sid);
        assert_eq!(pulled["turns"][0]["content"], "keep across close");
    });
}

// --- t1: engine-opaque session facade (P1 / AC1) ---
//
// Locks open/ensure/agent_chat_turn (+ session lifecycle entry) so callers cannot
// pass or branch on Host/Cursor/engine selection. API-level shield — not docs.

#[test]
fn t1_facade_signatures_are_engine_opaque_type_locked() {
    use crate::commands::ai_assistant::{
        agent_chat_turn_json, ensure_ai_assistant_session_json, open_ai_assistant_json,
        SESSION_FACADE_ENGINE_OPAQUE,
    };
    use crate::services::agent::r#loop::ChatTurnResult;
    use crate::services::agent::session::{self, SESSION_LIFECYCLE_ENGINE_OPAQUE};
    use serde_json::Value;

    assert!(
        SESSION_FACADE_ENGINE_OPAQUE,
        "facade must declare API-level engine opacity (not a comment-only convention)"
    );
    assert!(
        SESSION_LIFECYCLE_ENGINE_OPAQUE,
        "session lifecycle entry must declare API-level engine opacity"
    );

    // Type ascriptions fail to compile if an optional engine backdoor param is added.
    let _: fn(&str) -> Result<Value, String> = open_ai_assistant_json;
    let _: fn() -> Result<Value, String> = ensure_ai_assistant_session_json;
    let _: fn(&str, &str, Option<&str>) -> Result<ChatTurnResult, String> = agent_chat_turn_json;
    let _: fn(Option<&str>, Option<&str>) -> Result<session::Session, String> =
        session::create_session;
}

#[test]
fn t1_facade_happy_path_without_engine_params_and_payloads_leak_none() {
    with_cmd_sandbox(|| {
        use crate::commands::ai_assistant::value_exposes_engine_selection;

        let id = create_plan("t1门面无引擎");
        let open = open_ai_assistant_json(&id).expect("open without engine param");
        assert!(!open["session_id"].as_str().unwrap_or("").is_empty());
        assert!(
            !value_exposes_engine_selection(&open),
            "open payload must not expose engine fields for caller branching: {open}"
        );

        let ensured = ensure_ai_assistant_session_json().expect("ensure without engine param");
        assert!(!ensured["session_id"].as_str().unwrap_or("").is_empty());
        assert!(
            !value_exposes_engine_selection(&ensured),
            "ensure payload must not expose engine fields: {ensured}"
        );

        let sid = ensured["session_id"].as_str().unwrap();
        let turn = agent_chat_turn_json(sid, "hi t1", Some(&id)).expect("chat without engine");
        assert!(
            !value_exposes_engine_selection(&turn.body),
            "chat body must not expose engine fields: {}",
            turn.body
        );
        if let Some(ev) = &turn.emit_turn_completed {
            assert!(
                !value_exposes_engine_selection(ev),
                "turn-completed event must not expose engine fields: {ev}"
            );
        }

        // Binding call surface still reaches the same facade (engine invisible).
        assert_eq!(query_binding_json()["state"], "unbound");
        let pulled = get_ai_assistant_binding_json();
        assert!(
            !value_exposes_engine_selection(&pulled),
            "binding pull must not expose engine fields: {pulled}"
        );
    });
}

#[test]
fn record_ai_assistant_timing_persists_safe_ui_receive_duration() {
    with_cmd_sandbox(|| {
        record_ai_assistant_timing(
            "ui_trace_12345678".into(),
            "response_received".into(),
            3,
            Some(42),
        )
        .expect("record UI timing");

        let path = crate::services::agent::diagnostics::diagnostic_log_path()
            .expect("diagnostic path");
        let log = fs::read_to_string(path).expect("diagnostic log");
        let event: serde_json::Value =
            serde_json::from_str(log.lines().last().expect("diagnostic event"))
                .expect("JSONL event");

        assert_eq!(event["component"], "assistant.ui");
        assert_eq!(event["event"], "turn.response_received");
        assert_eq!(event["trace_id"], "ui_trace_12345678");
        assert_eq!(event["elapsed_ms"], 3);
        assert_eq!(event["fields"]["input_to_response_ms"], 42);
        assert!(event.get("message").is_none() && event.get("content").is_none());
    });
}

#[test]
fn t1_session_lifecycle_entry_has_no_engine_selection_api() {
    with_cmd_sandbox(|| {
        use crate::commands::ai_assistant::value_exposes_engine_selection;
        use crate::services::agent::session::{self, SESSION_LIFECYCLE_ENGINE_OPAQUE};

        assert!(SESSION_LIFECYCLE_ENGINE_OPAQUE);
        // Lifecycle create accepts only optional master/title — no engine.
        let sess = session::create_session(Some("master_x"), Some("title_x")).expect("create");
        let serialized = serde_json::to_value(&sess).expect("serialize session");
        assert!(
            !value_exposes_engine_selection(&serialized),
            "session record must not carry engine selection fields: {serialized}"
        );
        assert!(serialized.get("engine").is_none());
        assert!(serialized.get("engine_type").is_none());
        assert!(serialized.get("engineType").is_none());
        assert!(serialized.get("assistant_engine").is_none());

        // Helper itself recognizes forbidden keys (API shield, not convention).
        assert!(value_exposes_engine_selection(&json!({ "engine": "host" })));
        assert!(value_exposes_engine_selection(&json!({ "engine_type": "cursor" })));
        assert!(value_exposes_engine_selection(&json!({ "engineType": "host" })));
        assert!(value_exposes_engine_selection(&json!({ "assistant_engine": "cursor" })));
        assert!(value_exposes_engine_selection(
            &json!({ "payload": { "engine": "host" } })
        ));
        assert!(!value_exposes_engine_selection(&json!({
            "session_id": "sess_x",
            "window_label": "ai-assistant"
        })));
    });
}

#[test]
fn set_binding_old_app_keys_notes_and_todo_task_fail() {
    with_cmd_sandbox(|| {
        for key in ["notes", "todo_task"] {
            let result = set_binding_json(json!({ "key": key }));
            assert_eq!(result["ok"], false, "key={key}");
            assert_eq!(result["code"], "unknown_key", "key={key}");
            assert_eq!(result["state"], "unbound", "key={key}");
        }
    });
}

#[test]
fn set_binding_empty_key_is_invalid_not_workbench() {
    with_cmd_sandbox(|| {
        let empty = set_binding_json(json!({ "key": "" }));
        assert_eq!(empty["ok"], false);
        assert_eq!(empty["code"], "set_invalid");
        assert_eq!(empty["state"], "unbound");

        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        assert_eq!(
            set_binding_json(json!({ "key": SEEDED_BUSINESS_KEY }))["ok"],
            true
        );
        let again = set_binding_json(json!({ "key": "" }));
        assert_eq!(again["ok"], false);
        assert_eq!(again["code"], "set_invalid");
        assert_eq!(query_binding_json()["state"], "bound");
    });
}

#[test]
fn list_select_create_chat_sessions_for_home_history() {
    with_cmd_sandbox(|| {
        let empty = list_chat_sessions_json().expect("list empty");
        assert_eq!(empty["sessions"].as_array().map(|a| a.len()), Some(0));

        let first = create_chat_session_json().expect("create first");
        let first_id = first["session_id"].as_str().expect("first id").to_string();
        session::append_turn(
            &first_id,
            Turn {
                role: "user".into(),
                content: Some("Hello from first".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append");

        let second = create_chat_session_json().expect("create second");
        let second_id = second["session_id"].as_str().expect("second id").to_string();
        assert_ne!(first_id, second_id);

        let listed = list_chat_sessions_json().expect("list two");
        let sessions = listed["sessions"].as_array().expect("sessions array");
        assert_eq!(sessions.len(), 2);
        assert_eq!(listed["current_session_id"], second_id);
        assert!(sessions.iter().any(|s| s["title"] == "Hello from first"));
        assert!(
            sessions.iter().any(|s| {
                s["title"].as_str().is_some_and(|title| {
                    title == "New conversation" || is_session_when_label(title)
                })
            }),
            "untitled session should show date-time, got {sessions:?}"
        );

        let selected = select_chat_session_json(&first_id).expect("select first");
        assert_eq!(selected["session_id"], first_id);
        let turns = selected["turns"].as_array().expect("turns");
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0]["content"], "Hello from first");

        let after = list_chat_sessions_json().expect("list after select");
        assert_eq!(after["current_session_id"], first_id);

        for _ in 0..20 {
            create_chat_session_json().expect("fill list");
        }
        let capped = list_chat_sessions_json().expect("list cap");
        assert_eq!(
            capped["sessions"].as_array().map(|a| a.len()),
            Some(20),
            "Home history lists the 20 most recent sessions"
        );
    });
}

#[test]
fn set_binding_cursor_ide_is_not_an_app_key() {
    with_cmd_sandbox(|| {
        let result = set_binding_json(json!({ "key": "cursor_ide" }));
        assert_eq!(result["ok"], false);
        assert_eq!(result["code"], "unknown_key");
        assert_eq!(result["state"], "unbound");
    });
}
