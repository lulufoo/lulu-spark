//! Present / open / close / chat-turn shell tests.

use super::support::*;


#[test]
fn ensure_chat_session_idle_creates_session() {
    with_sandbox(|| {
        let v = r#loop::ensure_chat_session_core().expect("ensure");
        assert!(v["session_id"].as_str().unwrap().starts_with("sess_"));
        assert!(v.get("bound_master_task_id").is_none());
        assert_eq!(v["window_label"], "ai-assistant");
        assert_ne!(v["busy"], true);
    });
}

#[test]
fn agent_chat_turn_happy_path_emits_turn_completed() {
    with_sandbox(|| {
        let master = create_bound_plan("对话");
        let mock = spawn_scripted_llm(vec![assistant_text("收到")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["busy"], false);
        assert!(result.body.get("bound_master_task_id").is_none());
        assert!(result.body["reply_text"].as_str().unwrap().contains("收到"));
        let ev = result.emit_turn_completed.expect("must emit");
        assert_eq!(ev["event"], EVENT_TURN_COMPLETED);
        assert_eq!(ev["payload"]["session_id"], sid);
        assert_eq!(ev["payload"]["wrote"], false);
        assert_eq!(ev["payload"]["terminal"], "none");
        assert!(ev["payload"].get("bound_master_task_id").is_none());
    });
}

#[test]
fn agent_chat_turn_ignores_master_arg_uses_binding_ctx() {
    with_sandbox(|| {
        let a = create_bound_plan("绑定A");
        let b = create_bound_plan("其它B");
        let mock = spawn_scripted_llm(vec![assistant_text("ok")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&a);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        // Client master arg is ignored; Binding.tools ctx drives execution.
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&b)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert!(result.emit_turn_completed.is_some());
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
        assert!(result.body.get("bound_master_task_id").is_none());
    });
}

#[test]
fn close_window_does_not_abort_emit_still_available() {
    with_sandbox(|| {
        let master = create_bound_plan("关窗");
        let mock = spawn_scripted_llm(vec![assistant_text("跑完了")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "继续", Some(&master)).unwrap();
        assert!(result.emit_turn_completed.is_some());
        assert_eq!(result.body["reply_text"], "跑完了");
    });
}

#[test]
fn event_name_is_literally_ai_assistant_turn_completed() {
    assert_eq!(EVENT_TURN_COMPLETED, "ai-assistant:turn-completed");
}

// --- Present shell surface: Present≠Set≠bound; not Binding Contract ops (t4) ---

#[test]
fn binding_contract_ops_exclude_present_and_open() {
    with_sandbox(|| {
        let ops = r#loop::binding_contract_ops();
        let normalized: Vec<String> = ops
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        for required in ["set", "reset", "query", "execute"] {
            assert!(
                normalized.iter().any(|op| op == required),
                "Binding Contract ops must include {required}; got {ops:?}"
            );
        }
        assert!(
            normalized.iter().any(|op| op.contains("callback")),
            "Binding Contract ops must include callbacks surface; got {ops:?}"
        );
        for forbidden in ["present", "open"] {
            assert!(
                normalized
                    .iter()
                    .all(|op| op != forbidden && !op.contains(forbidden)),
                "Present/Open must not appear in Binding Contract ops: {ops:?}"
            );
        }
    });
}

#[test]
fn present_ai_assistant_maps_to_shell_focus_without_set() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        let outcome = r#loop::present_ai_assistant_core().expect("Present must succeed");
        assert_eq!(
            outcome.window_label, r#loop::WINDOW_LABEL,
            "Present maps to existing ai-assistant shell focus surface"
        );
        assert_eq!(
            outcome.surface, "Present",
            "semantic surface name is Present (not a Binding Contract op)"
        );
        assert_eq!(
            outcome.entry_id, "ai-assistant",
            "T3 PresentOutcome must carry entry_id for shell openEntry"
        );
        assert_query_unbound(&r#loop::query_binding());
        let err = r#loop::execute_binding().expect_err("Present alone must not enable execute");
        assert_eq!(err.as_code(), "rejected_unbound");
    });
}

#[test]
fn present_preserves_binding_state_unbound_and_bound() {
    with_sandbox(|| {
        // unbound → Present → still unbound
        assert_query_unbound(&r#loop::query_binding());
        r#loop::present_ai_assistant_core().expect("Present unbound");
        assert_query_unbound(&r#loop::query_binding());

        // bound → Present → still bound (same generation)
        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::present_ai_assistant_core().expect("Present while bound");
        let after = r#loop::query_binding();
        assert_eq!(
            after, before,
            "Present must not change query state while bound"
        );
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.is_empty(),
            "Present must not emit Binding lifecycle events: {events:?}"
        );
    });
}

#[test]
fn present_path_does_not_write_contract_business_binding_primary_key() {
    with_sandbox(|| {
        let master = create_bound_plan("Present无契约业务主键");
        // Present core must not smear a business primary key into Binding Contract state.
        let _ = r#loop::present_ai_assistant_core().expect("Present");
        assert_query_unbound(&r#loop::query_binding());
        assert_query_is_business_agnostic(&r#loop::query_binding());

        // Legacy open may still carry a master id for shell UX, but must not imply Set/bound.
        let _ = r#loop::ensure_chat_session_core();
        assert_eq!(
            r#loop::binding_state(),
            "unbound",
            "open/Present path must not imply Binding Contract bound"
        );
        let q = r#loop::query_binding();
        assert_query_is_business_agnostic(&q);
        let encoded = serde_json::to_value(&q).unwrap();
        assert!(
            !encoded
                .as_object()
                .unwrap()
                .keys()
                .any(|k| k.contains("master_task")),
            "contract query must not expose business binding primary key: {encoded}"
        );
    });
}

#[test]
fn shell_close_is_not_reset_and_does_not_emit_on_unbound() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::shell_close_core().expect("shell close (关壳) must be callable");
        let after = r#loop::query_binding();
        assert_eq!(
            after, before,
            "关壳 ≠ Reset: binding state must stay bound until explicit Reset"
        );
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "关壳 must not emit onUnbound; Reset must be explicit: {events:?}"
        );

        // Explicit Reset still works after shell close.
        r#loop::reset_binding().expect("explicit Reset");
        assert_query_unbound(&r#loop::query_binding());
    });
}

#[test]
fn present_command_json_does_not_set_binding() {
    with_sandbox(|| {
        use crate::commands::ai_assistant::{
            execute_binding_json, present_ai_assistant_json, query_binding_json, set_binding_json,
        };
        let presented = present_ai_assistant_json().expect("Present command");
        assert_eq!(presented["surface"], "Present");
        assert_eq!(presented["window_label"], r#loop::WINDOW_LABEL);
        assert_eq!(presented["entry_id"], "ai-assistant");
        let q = query_binding_json();
        assert_eq!(q["state"], "unbound");
        let exec = execute_binding_json();
        assert_eq!(exec["ok"], false);
        assert_eq!(exec["code"], "rejected_unbound");

        // Present after Set still leaves bound and does not replace.
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        let set = set_binding_json(json!({ "key": SEEDED_BUSINESS_KEY }));
        assert_eq!(set["ok"], true);
        let gen_before = query_binding_json()["generation"].clone();
        let _ = present_ai_assistant_json().expect("Present while bound");
        assert_eq!(query_binding_json()["state"], "bound");
        assert_eq!(query_binding_json()["generation"], gen_before);
    });
}

#[test]
fn t1_agent_chat_turn_allows_when_generation_current() {
    with_sandbox(|| {
        let master = create_bound_plan("t1-gen-ok");
        let mock = spawn_scripted_llm(vec![assistant_text("世代有效")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        assert!(r#loop::is_binding_generation_current(gen));
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert!(result.body["reply_text"].as_str().unwrap().contains("世代有效"));
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
    });
}

#[test]
fn t1_agent_chat_turn_after_reset_does_not_invoke_llm() {
    with_sandbox(|| {
        let master = create_bound_plan("t1-gen-reset");
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        r#loop::reset_binding().expect("Reset");
        assert!(!r#loop::is_binding_generation_current(gen));

        let result = r#loop::agent_chat_turn_core(&sid, "继续", Some(&master)).unwrap();
        assert_ne!(
            result.body["terminal"],
            "none",
            "after Reset, chat must not continue as a successful executable turn"
        );
        assert_eq!(
            mock.hits.lock().unwrap().len(),
            0,
            "generation invalidation must not call LLM / old Tools+Prompt path"
        );
    });
}

#[test]
fn t4_shell_close_core_is_not_defensive_cut() {
    with_sandbox(|| {
        let master = create_bound_plan("t4-shell-close");
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation.expect("gen");
        r#loop::clear_lifecycle_events_for_tests();

        // shell_close_core is defined only in loop.rs; shell_close_json only wraps it.
        r#loop::shell_close_core().expect("关壳");
        assert_eq!(
            r#loop::query_binding(),
            before,
            "关壳 ≠ 切断: binding must remain observable as before explicit Reset"
        );
        assert_eq!(live_session_id().as_deref(), Some(sid.as_str()));
        assert!(r#loop::is_binding_generation_current(gen));
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "shell_close must not emit onUnbound: {events:?}"
        );

        // Explicit Reset still works after shell close (pre-Reset rules).
        r#loop::reset_binding().expect("explicit Reset after shell close");
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(live_session_id(), None);
    });
}

#[test]
fn t5_shell_close_does_not_record_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::shell_close_core().expect("关壳");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs
                .iter()
                .all(|e| e.event != "ai-assistant:binding-changed"),
            "关壳 ≠ 切断: must not emit shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t6_shell_close_is_not_cut_acceptance() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-shell-close");
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::shell_close_core().expect("shell close");
        // 关壳 ≠ 切断：仍 bound / generation current / executable
        assert_eq!(r#loop::binding_state(), "bound");
        assert!(r#loop::is_binding_generation_current(gen));
        r#loop::execute_binding().expect("shell_close must not cut execute");
        // Explicit Reset remains the primary leave path
        r#loop::reset_binding().expect("explicit Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t3_host_facade_open_ensure_chat_works_host_only() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-host-only");
        let mock = spawn_scripted_llm(vec![assistant_text("Host-only facade session")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);

        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        assert!(!sid.is_empty());

        let ensure = r#loop::ensure_chat_session_core().unwrap();
        assert_eq!(ensure["session_id"], sid);

        let result = r#loop::agent_chat_turn_core(&sid, "继续", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert!(result.emit_turn_completed.is_some());
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap_or("")
                .contains("Host-only facade session")
        );
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t4_hub_and_shell_close_are_not_reset_paths() {
    let shell = repo_file("frontend/src/home-entry-shell/shell.tsx");
    let note_assistant = repo_file("frontend/src/notes/ui/assistant.tsx");
    let note_assistant_commands = repo_file("frontend/src/notes/commands/assistant.ts");
    assert!(
        !shell.contains("reset_binding")
            && !shell.contains("resetNotesBinding")
            && !shell.contains("resetTodosBinding"),
        "Hub small window must not Reset Binding"
    );
    assert!(
        !note_assistant.contains("reset_binding")
            && !note_assistant.contains("resetNotesBinding")
            && !note_assistant.contains("resetTodosBinding")
            && !note_assistant_commands.contains("reset_binding")
            && !note_assistant_commands.contains("resetNotesBinding")
            && !note_assistant_commands.contains("resetTodosBinding"),
        "note-assistant / close-shell path must not Reset Binding"
    );
}

#[test]
fn t4_main_and_sidebar_wire_notes_set_without_new_runtime() {
    let main = repo_file("frontend/src/boot.ts");
    let sidebar = repo_file("frontend/src/notes/ui/sidebar.tsx");
    let sidebar_commands = repo_file("frontend/src/notes/commands/sidebar.ts");
    let lifecycle = repo_file("frontend/src/todo-task/commands/lifecycle.ts");
    assert!(
        main.contains("setWorkbenchBinding")
            && !main.contains("buildNotesBinding")
            && !main.contains("resetNotesBinding"),
        "main.js must Set workbench at app shell and must not wire notes Set/Reset"
    );
    assert!(
        !sidebar.contains("buildNotesBinding") && !sidebar.contains("set_binding"),
        "sidebar.js selectDate path must not Set Binding"
    );
    assert!(
        !sidebar_commands.contains("buildNotesBinding") && !sidebar_commands.contains("set_binding"),
        "sidebar-commands.ts selectDate path must not Set Binding"
    );
    assert!(
        !main.contains("new Agent")
            && !sidebar.contains("new Agent")
            && !sidebar_commands.contains("new Agent"),
        "must not start a separate assistant runtime"
    );
    assert!(
        !lifecycle.contains("resetTodosBinding")
            && !lifecycle.contains("buildTodosBinding")
            && lifecycle.contains("notifyShellClose"),
        "todos-lifecycle must keep shell-close ≠ Reset and must not call deleted Binding helpers"
    );
}
