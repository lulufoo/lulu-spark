//! Flight table, busy gate, and in-flight cancel tests.

use super::support::*;


#[test]
fn replace_set_while_busy_clears_live_session() {
    with_sandbox(|| {
        let a = create_bound_plan("计划A");
        let b = create_bound_plan("计划B");
        arm_plan_binding(&a);
        let first = r#loop::ensure_chat_session_core().unwrap();
        let sid = first["session_id"].as_str().unwrap().to_string();
        r#loop::set_busy_for_tests(true);
        // Replace Set while busy still cuts the live session (clear-first).
        arm_plan_binding(&b);
        assert_eq!(
            live_session_id(),
            None,
            "replace Set clears live session even while busy"
        );
        // Legacy open_ai_assistant busy gate is retired; ensure may mint a new session.
        let second = r#loop::ensure_chat_session_core().unwrap();
        assert!(
            second["session_id"]
                .as_str()
                .unwrap()
                .starts_with("workbench_chat_")
        );
        assert_ne!(second["session_id"].as_str().unwrap(), sid);
        assert!(second.get("bound_master_task_id").is_none());
    });
}

#[test]
fn agent_chat_turn_busy_rejects_without_emit_or_user_turn() {
    with_sandbox(|| {
        let master = create_bound_plan("忙");
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let before = session::load_session(&sid).unwrap();
        let before_len = before.turns.len();
        r#loop::set_busy_for_tests(true);
        let result = r#loop::agent_chat_turn_core(&sid, "第二路", Some(&master)).unwrap();
        assert_eq!(result.body["busy"], true);
        assert!(result.emit_turn_completed.is_none());
        let after = session::load_session(&sid).unwrap();
        assert_eq!(after.turns.len(), before_len, "user must not enter session");
    });
}

#[test]
fn in_flight_chat_keeps_binding_despite_busy_open_without_tool_writes() {
    // Narrowed (P3): busy open still rejected; Host chat is text-only (wrote=false).
    with_sandbox(|| {
        let a = create_bound_plan("旧绑定");
                let mock = spawn_scripted_llm(vec![assistant_text("已理解改标题请求（无进程内写入）")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&a);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        r#loop::set_busy_for_tests(true);
        // Ensure while busy keeps the existing live session; does not Set Binding.
        let ensured = r#loop::ensure_chat_session_core().unwrap();
        assert_eq!(ensured["session_id"], sid);
        assert_eq!(ensured["busy"], true);
        assert!(ensured.get("bound_master_task_id").is_none());
        r#loop::set_busy_for_tests(false);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&a)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "none");
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t2_chat_session_identity_must_match_live_current() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-identity");
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let live = open["session_id"].as_str().unwrap().to_string();
        assert_eq!(live_session_id().as_deref(), Some(live.as_str()));

        // Stale / foreign session id must be rejected; must not re-pin runtime.
        let foreign = session::create_session().unwrap().session_id;
        let rejected = r#loop::agent_chat_turn_core(&foreign, "ping", Some(&master));
        match rejected {
            Ok(result) => {
                assert_ne!(
                    result.body["terminal"],
                    "none",
                    "mismatched session id must not continue as a successful executable turn"
                );
            }
            Err(_) => {} // hard reject is also acceptable
        }
        assert_eq!(
            live_session_id().as_deref(),
            Some(live.as_str()),
            "reject must not re-pin runtime current_session_id to the foreign id"
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 0);
    });
}

#[test]
fn t3_reset_while_busy_and_executing_cancels_both_inflight_paths() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::set_busy_for_tests(true);
        let err = r#loop::execute_binding_during(|| {
            assert_eq!(
                r#loop::get_ai_assistant_binding_core()["busy"],
                true,
                "chat in-flight must be observable as busy"
            );
            r#loop::reset_binding().expect("Reset");
            assert!(
                r#loop::is_execute_cancelled_for_tests(),
                "Reset must set execute cancel while executing"
            );
            assert!(
                r#loop::is_chat_cancelled_for_tests(),
                "Reset must set chat cancel while busy — not execute-only"
            );
        })
        .expect_err("in-flight execute must cancel");
        assert_eq!(err.as_code(), "reset_cancelled");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
    });
}

#[test]
fn t3_replace_set_while_busy_cancels_chat_and_interrupts_execute() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::set_busy_for_tests(true);
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        let err = r#loop::execute_binding_during(|| {
            r#loop::set_binding(binding_with_prompt("replaced-mid-flight")).expect("replace Set");
            assert!(
                !r#loop::is_binding_generation_current(old_gen),
                "replace Set must invalidate the in-flight execute generation"
            );
            assert!(
                r#loop::is_chat_cancelled_for_tests(),
                "replace Set must cancel in-flight chat while busy — not execute-only"
            );
            assert_eq!(live_session_id(), None);
        })
        .expect_err("in-flight execute must be interrupted by replace Set");
        // Generation-stale remains the distinguishable replace reject (T1); interrupt is required.
        assert_eq!(err.as_code(), "rejected_stale_generation");
    });
}

#[test]
fn t3_defensive_unbound_cancels_inflight_chat_symmetrically() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-defensive");
        let mock = spawn_llm_with_mid_then_response(
            || {
                r#loop::defensive_unbound().expect("defensive cut");
                assert!(
                    r#loop::is_chat_cancelled_for_tests(),
                    "defensive cut must cancel in-flight chat"
                );
                assert_eq!(r#loop::binding_state(), "unbound");
                assert_eq!(live_session_id(), None);
            },
            assistant_text("should-not-land"),
        );
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(result.body["wrote"], false);
        assert_eq!(
            session_turn_contents(&sid),
            turns_before,
            "defensive cancel reject must not append business turns"
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
    });
}

#[test]
fn t3_cancel_notice_is_not_cut_semantics() {
    with_sandbox(|| {
        // Cut semantics = cancel flags + generation invalidate + session clear.
        // Returning a notice/error body is allowed; writing notice turns is not the cut.
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::set_busy_for_tests(true);
        r#loop::reset_binding().expect("Reset");
        assert!(r#loop::is_chat_cancelled_for_tests());
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
        assert!(
            r#loop::query_binding().generation.is_none(),
            "cut must invalidate generation; notice turns are not a substitute"
        );
    });
}

#[test]
fn flights_same_session_and_cap_three_allow_select() {
    with_sandbox(|| {
        let a = r#loop::create_chat_session_core().unwrap();
        let a_id = a["session_id"].as_str().unwrap().to_string();
        let b = r#loop::create_chat_session_core().unwrap();
        let b_id = b["session_id"].as_str().unwrap().to_string();
        let c = r#loop::create_chat_session_core().unwrap();
        let c_id = c["session_id"].as_str().unwrap().to_string();
        let d = r#loop::create_chat_session_core().unwrap();
        let d_id = d["session_id"].as_str().unwrap().to_string();

        let sink = crate::agent::progress::noop_sink();
        let tid = crate::agent::diagnostics::TraceId::new();

        r#loop::select_chat_session_core(&a_id).unwrap();
        r#loop::try_begin_chat_turn(&a_id, &tid, sink.clone()).unwrap();
        let same = r#loop::try_begin_chat_turn(&a_id, &tid, sink.clone()).unwrap_err();
        assert_eq!(same.body["busy"], true);

        r#loop::select_chat_session_core(&b_id).unwrap();
        r#loop::try_begin_chat_turn(&b_id, &tid, sink.clone()).unwrap();
        r#loop::select_chat_session_core(&c_id).unwrap();
        r#loop::try_begin_chat_turn(&c_id, &tid, sink.clone()).unwrap();
        r#loop::select_chat_session_core(&d_id).unwrap();
        let fourth = r#loop::try_begin_chat_turn(&d_id, &tid, sink.clone()).unwrap_err();
        assert_eq!(fourth.body["busy"], true);

        r#loop::select_chat_session_core(&a_id).expect("select while other flights run");
        r#loop::end_chat_turn(&a_id);
        r#loop::try_begin_chat_turn(&a_id, &tid, sink).unwrap();
    });
}
