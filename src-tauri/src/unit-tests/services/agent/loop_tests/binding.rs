//! Binding Contract: set / reset / query / execute / MCP / ticket.

use super::support::*;


#[test]
fn set_binding_accepts_valid_tools_prompt_and_callbacks() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        let result = r#loop::set_binding(valid_binding());
        assert!(
            result.is_ok(),
            "valid Binding must pass Set validation, got {result:?}"
        );
        assert_ne!(
            result.err().map(|e| e.as_code()),
            Some("set_invalid")
        );
        assert_eq!(
            r#loop::binding_state(),
            "bound",
            "valid Set must enter subsequent bound path"
        );
    });
}

#[test]
fn binding_config_surface_is_generic_tools_prompt_callbacks() {
    with_sandbox(|| {
        let binding = valid_binding();
        let encoded = serde_json::to_value(&binding).expect("encode Binding");
        let obj = encoded.as_object().expect("Binding encodes as object");
        assert!(obj.contains_key("tools"), "tools slot required");
        assert!(obj.contains_key("prompt"), "prompt slot required");
        assert!(obj.contains_key("callbacks"), "callbacks slot required");
        assert!(
            !obj.contains_key("master_task_id"),
            "business field master_task_id must not be part of Binding contract surface"
        );
        assert!(
            !obj.contains_key("bound_master_task_id"),
            "business field bound_master_task_id must not be part of Binding contract surface"
        );
        r#loop::set_binding(binding).expect("generic Binding must Set successfully");
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn set_binding_allows_empty_callbacks_registry_when_slot_present() {
    with_sandbox(|| {
        let binding = r#loop::Binding {
            tools: applicable_tools(),
            prompt: applicable_prompt(),
            callbacks: json!({}),
        };
        assert!(r#loop::set_binding(binding).is_ok());
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn set_binding_allows_empty_tools_array_but_rejects_empty_prompt() {
    with_sandbox(|| {
        // L1+L2: empty tools array is legal (Host Agent business session tools empty).
        let empty_tools = r#loop::Binding {
            tools: json!([]),
            prompt: applicable_prompt(),
            callbacks: empty_callbacks_registry(),
        };
        r#loop::set_binding(empty_tools).expect("empty tools array must Set");
        assert_eq!(r#loop::binding_state(), "bound");
        r#loop::reset_binding().expect("reset");

        let empty_prompt = r#loop::Binding {
            tools: applicable_tools(),
            prompt: json!(""),
            callbacks: empty_callbacks_registry(),
        };
        let err = r#loop::set_binding(empty_prompt).expect_err("empty prompt must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn set_binding_rejects_missing_slot_keeps_prior_state() {
    with_sandbox(|| {
        // Missing any of tools / prompt / callbacks → set_invalid, stay unbound.
        for payload in [
            json!({ "prompt": "p", "callbacks": {} }),
            json!({ "tools": [{ "name": "t" }], "callbacks": {} }),
            json!({ "tools": [{ "name": "t" }], "prompt": "p" }),
        ] {
            let err = r#loop::try_set_binding_json(&payload)
                .expect_err("missing slot must fail Set");
            assert_eq!(err.as_code(), "set_invalid");
            assert_eq!(r#loop::binding_state(), "unbound");
        }

        // After a successful Set, illegal Set must not rewrite bound state.
        r#loop::set_binding(valid_binding()).expect("seed bound");
        assert_eq!(r#loop::binding_state(), "bound");
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p"
            // callbacks slot absent
        }))
        .expect_err("missing callbacks must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(
            r#loop::binding_state(),
            "bound",
            "illegal Set must keep prior bound state"
        );
    });
}

#[test]
fn set_binding_does_not_assemble_or_default_fill_from_business_fields() {
    with_sandbox(|| {
        // Business-only payload without tools/prompt must not be auto-completed into success.
        let err = r#loop::try_set_binding_json(&json!({
            "master_task_id": "task_business_only",
            "callbacks": {}
        }))
        .expect_err("Host must not fill tools/prompt from business fields");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");

        let err = r#loop::try_set_binding_json(&json!({
            "bound_master_task_id": "task_x",
            "tools": [],
            "prompt": "",
            "callbacks": {}
        }))
        .expect_err("empty tools/prompt must not succeed via business-field side path");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn set_then_query_returns_bound_business_agnostic_summary() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        r#loop::set_binding(binding_with_prompt("secret-prompt-body")).expect("Set");
        let q = r#loop::query_binding();
        assert_query_bound(&q);
        assert_query_is_business_agnostic(&q);
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(
            r#loop::current_binding_slot_count(),
            1,
            "after Set there must be exactly one current Binding"
        );
    });
}

#[test]
fn reset_bound_to_unbound_discards_current_binding() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation.expect("bound gen");

        r#loop::reset_binding().expect("Reset");
        let after = r#loop::query_binding();
        assert_query_unbound(&after);
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(
            r#loop::current_binding_slot_count(),
            0,
            "Reset must discard the current Binding reference"
        );
        assert!(
            !r#loop::is_binding_generation_current(gen),
            "old Binding generation must be discarded after Reset"
        );
    });
}

#[test]
fn set_on_bound_atomically_replaces_and_invalidates_old_binding() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("prompt-v1")).expect("first Set");
        let first = r#loop::query_binding();
        assert_query_bound(&first);
        let old_gen = first.generation.expect("old gen");

        r#loop::set_binding(binding_with_prompt("prompt-v2")).expect("replace Set");
        let second = r#loop::query_binding();
        assert_query_bound(&second);
        assert_query_is_business_agnostic(&second);
        let new_gen = second.generation.expect("new gen");
        assert_ne!(old_gen, new_gen, "replace Set must mint a new binding generation");
        assert!(
            !r#loop::is_binding_generation_current(old_gen),
            "old Binding must be immediately invalid after replace Set"
        );
        assert!(r#loop::is_binding_generation_current(new_gen));
        assert_eq!(
            r#loop::current_binding_slot_count(),
            1,
            "replace must leave exactly one current Binding"
        );
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn reset_already_unbound_is_idempotent_success() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        r#loop::reset_binding().expect("Reset unbound must succeed");
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::reset_binding().expect("second Reset unbound must succeed");
        assert_query_unbound(&r#loop::query_binding());
    });
}

#[test]
fn host_runtime_reset_starts_unbound_before_successful_set() {
    with_sandbox(|| {
        // with_sandbox already calls reset_runtime_for_tests (Host runtime reset).
        let q = r#loop::query_binding();
        assert_query_unbound(&q);
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::reset_runtime_for_tests();
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn query_does_not_change_binding_state() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        let _ = r#loop::query_binding();
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation;
        let slot = r#loop::current_binding_slot_count();

        let mid = r#loop::query_binding();
        assert_eq!(mid.state, before.state);
        assert_eq!(mid.generation, gen);
        assert_eq!(r#loop::current_binding_slot_count(), slot);
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn illegal_set_does_not_transition_to_bound() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        // Legacy/null tools payload remains illegal; empty array is legal elsewhere (L1+L2).
        let err = r#loop::try_set_binding_json(&json!({
            "tools": null,
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("illegal Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::set_binding(valid_binding()).expect("seed bound");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation;

        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p"
            // callbacks missing
        }))
        .expect_err("illegal Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        let after = r#loop::query_binding();
        assert_query_bound(&after);
        assert_eq!(after.generation, gen, "illegal Set must not rewrite current Binding");
        assert_eq!(r#loop::current_binding_slot_count(), 1);
    });
}

#[test]
fn host_serializes_set_reset_at_most_one_current_binding() {
    with_sandbox(|| {
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let max_slots = Arc::new(Mutex::new(0usize));
        let errors = Arc::new(Mutex::new(Vec::new()));

        let mut handles = Vec::new();
        for i in 0..8 {
            let barrier = barrier.clone();
            let max_slots = max_slots.clone();
            let errors = errors.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                for round in 0..40 {
                    let binding = binding_with_prompt(&format!("prompt-{i}-{round}"));
                    if let Err(e) = r#loop::set_binding(binding) {
                        errors
                            .lock()
                            .unwrap()
                            .push(format!("set failed: {}", e.as_code()));
                    }
                    let (q_after_set, slots) = r#loop::query_binding_snapshot();
                    {
                        let mut m = max_slots.lock().unwrap();
                        if slots > *m {
                            *m = slots;
                        }
                    }
                    if slots > 1 {
                        errors
                            .lock()
                            .unwrap()
                            .push(format!("observed {slots} current bindings"));
                    }
                    if q_after_set.state == "bound" && slots != 1 {
                        errors.lock().unwrap().push(format!(
                            "query bound but slot_count={slots}"
                        ));
                    }
                    if q_after_set.state == "unbound" && slots != 0 {
                        errors.lock().unwrap().push(format!(
                            "query unbound but slot_count={slots}"
                        ));
                    }
                    if round % 2 == 0 {
                        if let Err(_) = r#loop::reset_binding() {
                            errors.lock().unwrap().push("reset failed".into());
                        }
                    }
                    let (q, slots_after) = r#loop::query_binding_snapshot();
                    match q.state {
                        "unbound" => {
                            if slots_after != 0 {
                                errors.lock().unwrap().push(format!(
                                    "query unbound but slot_count={slots_after}"
                                ));
                            }
                        }
                        "bound" => {
                            if slots_after != 1 {
                                errors.lock().unwrap().push(format!(
                                    "query bound but slot_count={slots_after}"
                                ));
                            }
                        }
                        other => errors
                            .lock()
                            .unwrap()
                            .push(format!("unexpected state {other}")),
                    }
                }
            }));
        }

        for h in handles {
            h.join().expect("thread join");
        }
        let errs = errors.lock().unwrap().clone();
        assert!(
            errs.is_empty(),
            "serialization / dual-binding violations: {errs:?}"
        );
        assert!(
            *max_slots.lock().unwrap() <= 1,
            "must never observe more than one current Binding"
        );
        // Final state is well-formed: 0 or 1 slot matching query.
        let (final_q, final_slots) = r#loop::query_binding_snapshot();
        match final_q.state {
            "unbound" => assert_eq!(final_slots, 0),
            "bound" => assert_eq!(final_slots, 1),
            other => panic!("unexpected final state {other}"),
        }
    });
}

#[test]
fn execute_when_bound_uses_current_binding_tools_and_prompt() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("prompt-exec-v1")).expect("Set");
        let out = r#loop::execute_binding().expect("bound execute must succeed");
        assert_eq!(out.applied_prompt, json!("prompt-exec-v1"));
        assert_eq!(out.applied_tools, applicable_tools());
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn set_success_emits_on_bound_synchronously() {
    with_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(valid_binding()).expect("Set");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events), vec!["onBound"]);
        assert_payload_contract_only(&events[0]);
        assert!(events[0].category.is_none());
    });
}

#[test]
fn reset_to_unbound_emits_on_unbound() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events), vec!["onUnbound"]);
        assert_payload_contract_only(&events[0]);
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn replace_set_emits_on_unbound_then_on_bound_and_execute_uses_new() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("prompt-old")).expect("first Set");
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::set_binding(binding_with_prompt("prompt-new")).expect("replace Set");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(
            event_names(&events),
            vec!["onUnbound", "onBound"],
            "D1 replace sequence must be onUnbound → onBound"
        );
        for ev in &events {
            assert_payload_contract_only(ev);
        }
        assert!(!r#loop::is_binding_generation_current(old_gen));

        let out = r#loop::execute_binding().expect("execute after replace");
        assert_eq!(out.applied_prompt, json!("prompt-new"));
        assert_ne!(out.applied_prompt, json!("prompt-old"));
    });
}

#[test]
fn lifecycle_callbacks_delivered_synchronously_in_call_path_order() {
    with_sandbox(|| {
        let order = Arc::new(Mutex::new(Vec::new()));
        let order_c = order.clone();
        r#loop::set_lifecycle_listener_for_tests(Some(Box::new(move |ev| {
            order_c.lock().unwrap().push(ev.event.to_string());
        })));

        r#loop::set_binding(binding_with_prompt("a")).expect("Set");
        // Listener saw onBound before set_binding returned (sync).
        assert_eq!(order.lock().unwrap().clone(), vec!["onBound".to_string()]);

        r#loop::set_binding(binding_with_prompt("b")).expect("replace");
        assert_eq!(
            order.lock().unwrap().clone(),
            vec![
                "onBound".to_string(),
                "onUnbound".to_string(),
                "onBound".to_string()
            ]
        );
    });
}

#[test]
fn mid_execute_reset_strategy_a_unbounds_cancels_and_emits() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("in-flight")).expect("Set");
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::clear_lifecycle_events_for_tests();

        let err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset during execute");
            assert_eq!(
                r#loop::binding_state(),
                "unbound",
                "strategy A: immediately unbound"
            );
            assert!(!r#loop::is_binding_generation_current(gen));
        })
        .expect_err("in-flight execute must cancel/fail");
        assert_eq!(err.as_code(), "reset_cancelled");
        assert_eq!(r#loop::binding_state(), "unbound");

        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onUnbound"),
            "must observe onUnbound: {events:?}"
        );
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("must observe onError(reset_cancelled)");
        assert_eq!(err_ev.category, Some("reset_cancelled"));
        assert_payload_contract_only(err_ev);

        // Order: unbound first, then cancel error on the execute path.
        let names = event_names(&events);
        let i_unbound = names.iter().position(|n| *n == "onUnbound").unwrap();
        let i_err = names.iter().position(|n| *n == "onError").unwrap();
        assert!(
            i_unbound < i_err,
            "onUnbound before onError(reset_cancelled): {names:?}"
        );
    });
}

#[test]
fn reset_already_unbound_does_not_repeat_on_unbound() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("idempotent Reset");
        r#loop::reset_binding().expect("second idempotent Reset");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "idempotent Reset must not emit onUnbound: {events:?}"
        );
    });
}

#[test]
fn no_listener_still_advances_state_machine() {
    with_sandbox(|| {
        r#loop::set_lifecycle_listener_for_tests(None);
        r#loop::set_binding(valid_binding()).expect("Set without listener");
        assert_eq!(r#loop::binding_state(), "bound");
        r#loop::reset_binding().expect("Reset without listener");
        assert_eq!(r#loop::binding_state(), "unbound");
        // Events are still recorded (emission ≠ requiring a listener).
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onBound"),
            "events still emitted without listener: {events:?}"
        );
        assert!(
            events.iter().any(|e| e.event == "onUnbound"),
            "events still emitted without listener: {events:?}"
        );
    });
}

#[test]
fn execute_when_unbound_hard_fails_with_rejected_unbound() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("unbound execute must hard-fail");
        assert_eq!(err.as_code(), "rejected_unbound");
        let events = r#loop::drain_lifecycle_events();
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("onError(rejected_unbound)");
        assert_eq!(err_ev.category, Some("rejected_unbound"));
        assert_payload_contract_only(err_ev);
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn execute_after_reset_rejects_and_old_config_not_reused() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("old-only")).expect("Set");
        let before = r#loop::execute_binding().expect("execute while bound");
        assert_eq!(before.applied_prompt, json!("old-only"));

        r#loop::reset_binding().expect("Reset");
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("execute after Reset must reject");
        assert_eq!(err.as_code(), "rejected_unbound");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("rejected_unbound")),
            "{events:?}"
        );
        // No successful apply of old prompt after Reset.
        assert!(
            events
                .iter()
                .all(|e| e.event != "onBound"),
            "must not re-bind via execute: {events:?}"
        );
    });
}

#[test]
fn present_without_set_leaves_unbound_and_execute_rejects() {
    with_sandbox(|| {
        let master = create_bound_plan("仅Present");
        let _ = r#loop::open_ai_assistant_core(&master).expect("Present/open shell");
        assert_eq!(
            r#loop::binding_state(),
            "unbound",
            "Present/open must not imply Binding Contract bound"
        );
        let err = r#loop::execute_binding().expect_err("execute without Set");
        assert_eq!(err.as_code(), "rejected_unbound");
    });
}

#[test]
fn listener_panic_does_not_rollback_state_and_emits_callback_failed() {
    with_sandbox(|| {
        r#loop::set_lifecycle_listener_for_tests(Some(Box::new(|_| {
            panic!("listener boom");
        })));
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(valid_binding()).expect("Set must succeed despite listener panic");
        assert_eq!(
            r#loop::binding_state(),
            "bound",
            "listener failure must not roll back state machine"
        );
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onBound"),
            "onBound still emitted: {events:?}"
        );
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError" && e.category == Some("callback_failed"))
            .expect("onError(callback_failed)");
        assert_payload_contract_only(err_ev);
    });
}

#[test]
fn callback_payload_is_event_plus_category_only() {
    with_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(binding_with_prompt("secret-prompt-body")).expect("Set");
        let _ = r#loop::execute_binding().expect("bound ok");
        r#loop::reset_binding().expect("Reset");
        let _ = r#loop::execute_binding().expect_err("rejected");
        for ev in r#loop::drain_lifecycle_events() {
            assert_payload_contract_only(&ev);
        }
    });
}

#[test]
fn j1_1_legal_set_on_bound_execute_reset_rejects() {
    with_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(j1_generic_binding_fixture("j1-1-prompt")).expect("Set legal");
        let events_set = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events_set), vec!["onBound"]);
        assert_payload_contract_only(&events_set[0]);

        let out = r#loop::execute_binding().expect("execute while bound");
        assert_eq!(out.applied_prompt, json!("j1-1-prompt"));
        assert_eq!(
            out.applied_tools,
            json!([{ "name": "fixture_tool", "handle": "opaque-fixture-tool" }])
        );

        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let events_reset = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events_reset), vec!["onUnbound"]);
        assert_eq!(r#loop::binding_state(), "unbound");

        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("execute after Reset must reject");
        assert_eq!(err.as_code(), "rejected_unbound");
        let events_rej = r#loop::drain_lifecycle_events();
        let err_ev = events_rej
            .iter()
            .find(|e| e.event == "onError")
            .expect("onError(rejected_unbound)");
        assert_eq!(err_ev.category, Some("rejected_unbound"));
        assert_payload_contract_only(err_ev);
    });
}

#[test]
fn j1_2_illegal_set_keeps_state_no_on_bound_emits_set_invalid() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();

        // Missing content / empty tools — B1 illegal Set.
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [],
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("illegal Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");

        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onBound"),
            "illegal Set must not emit onBound: {events:?}"
        );
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("J1-(2): onError(set_invalid) must be observable");
        assert_eq!(err_ev.category, Some("set_invalid"));
        assert_payload_contract_only(err_ev);

        // After bound, illegal Set must keep prior state and still emit set_invalid.
        r#loop::set_binding(j1_generic_binding_fixture("keep")).expect("seed");
        let gen = r#loop::query_binding().generation;
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p"
            // callbacks missing
        }))
        .expect_err("missing slot");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::query_binding().generation, gen);
        let events2 = r#loop::drain_lifecycle_events();
        assert!(events2.iter().all(|e| e.event != "onBound"));
        assert!(
            events2
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("set_invalid")),
            "{events2:?}"
        );
    });
}

#[test]
fn j1_3_replace_set_on_unbound_then_on_bound_execute_uses_new() {
    with_sandbox(|| {
        r#loop::set_binding(j1_generic_binding_fixture("old-j1-3")).expect("first");
        let old_gen = r#loop::query_binding().generation.expect("old");
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::set_binding(j1_generic_binding_fixture("new-j1-3")).expect("replace");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events), vec!["onUnbound", "onBound"]);
        assert!(!r#loop::is_binding_generation_current(old_gen));

        let out = r#loop::execute_binding().expect("execute uses new");
        assert_eq!(out.applied_prompt, json!("new-j1-3"));
        assert_ne!(out.applied_prompt, json!("old-j1-3"));
    });
}

#[test]
fn j1_4_mid_execute_reset_unbounds_cancels_with_on_error() {
    with_sandbox(|| {
        r#loop::set_binding(j1_generic_binding_fixture("in-flight-j1-4")).expect("Set");
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::clear_lifecycle_events_for_tests();

        let err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset mid-execute");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(!r#loop::is_binding_generation_current(gen));
        })
        .expect_err("must cancel");
        assert_eq!(err.as_code(), "reset_cancelled");

        let events = r#loop::drain_lifecycle_events();
        assert!(events.iter().any(|e| e.event == "onUnbound"), "{events:?}");
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("onError(reset_cancelled)");
        assert_eq!(err_ev.category, Some("reset_cancelled"));
    });
}

#[test]
fn j1_5_contract_states_tools_prompt_callbacks_present_not_bound() {
    with_sandbox(|| {
        let binding = j1_generic_binding_fixture("j1-5");
        let encoded = serde_json::to_value(&binding).unwrap();
        let obj = encoded.as_object().unwrap();
        assert!(obj.contains_key("tools"));
        assert!(obj.contains_key("prompt"));
        assert!(obj.contains_key("callbacks"));
        assert!(!obj.contains_key("master_task_id"));
        assert!(!obj.contains_key("bound_master_task_id"));
        assert!(!obj.contains_key("todo_id"));

        // Present / 壳打开 ≠ bound
        let _ = r#loop::present_ai_assistant_core().expect("Present");
        assert_eq!(r#loop::binding_state(), "unbound");
        let err = r#loop::execute_binding().expect_err("Present alone ≠ bound");
        assert_eq!(err.as_code(), "rejected_unbound");

        let ops = r#loop::binding_contract_ops();
        let lower: Vec<_> = ops.iter().map(|s| s.to_ascii_lowercase()).collect();
        assert!(lower.iter().any(|o| o == "set"));
        assert!(lower.iter().any(|o| o == "reset"));
        assert!(lower.iter().any(|o| o == "query"));
        assert!(lower.iter().any(|o| o == "execute"));
        assert!(lower.iter().any(|o| o.contains("callback")));
        assert!(lower.iter().all(|o| o != "present" && !o.contains("present")));
        assert!(lower.iter().all(|o| o != "open" && !o.contains("open")));
    });
}

#[test]
fn j1_execute_gate_bound_success_and_unbound_reject() {
    with_sandbox(|| {
        // unbound → reject + onError(rejected_unbound)
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("unbound hard-fail");
        assert_eq!(err.as_code(), "rejected_unbound");
        assert!(
            r#loop::drain_lifecycle_events()
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("rejected_unbound"))
        );

        // bound → success with current Binding as sole config
        r#loop::set_binding(j1_generic_binding_fixture("gate-ok")).expect("Set");
        let out = r#loop::execute_binding().expect("bound success");
        assert_eq!(out.applied_prompt, json!("gate-ok"));
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn j1_h1_kernel_api_fixture_driver_not_business_ui() {
    with_sandbox(|| {
        // H1: acceptance driven by kernel API + generic Binding fixture only.
        // Prove Present observation does not substitute for contract Set.
        let _ = r#loop::present_ai_assistant_core().expect("Present observable");
        assert_eq!(r#loop::binding_state(), "unbound");

        r#loop::set_binding(j1_generic_binding_fixture("h1-kernel")).expect("kernel Set");
        assert_eq!(r#loop::binding_state(), "bound");
        let _ = r#loop::execute_binding().expect("kernel execute");
        r#loop::reset_binding().expect("kernel Reset");
        assert_eq!(r#loop::binding_state(), "unbound");

        // No business-page entry point invoked; fixture callbacks remain empty registry.
        let encoded = serde_json::to_value(j1_generic_binding_fixture("h1-kernel")).unwrap();
        assert_eq!(encoded["callbacks"], json!({}));
        assert!(encoded.get("master_task_id").is_none());
    });
}

#[test]
fn t1_execute_when_bound_and_generation_current_succeeds() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("gen-ok")).expect("Set");
        let gen = r#loop::query_binding().generation.expect("gen");
        assert!(
            r#loop::is_binding_generation_current(gen),
            "execute path requires a live generation"
        );
        let out = r#loop::execute_binding().expect("bound+current gen must succeed");
        assert_eq!(out.applied_prompt, json!("gen-ok"));
        assert!(r#loop::is_binding_generation_current(gen));
    });
}

#[test]
fn t1_replace_set_advances_generation_then_execute_under_new() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("old-gen")).expect("first");
        let old_gen = r#loop::query_binding().generation.expect("old");
        r#loop::set_binding(binding_with_prompt("new-gen")).expect("replace");
        let new_gen = r#loop::query_binding().generation.expect("new");
        assert_ne!(old_gen, new_gen);
        assert!(!r#loop::is_binding_generation_current(old_gen));
        assert!(r#loop::is_binding_generation_current(new_gen));
        let out = r#loop::execute_binding().expect("execute under new generation");
        assert_eq!(out.applied_prompt, json!("new-gen"));
        assert_ne!(out.applied_prompt, json!("old-gen"));
    });
}

#[test]
fn t1_mid_execute_replace_set_rejects_stale_generation() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("in-flight-old")).expect("Set");
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        r#loop::clear_lifecycle_events_for_tests();

        let err = r#loop::execute_binding_during(|| {
            r#loop::set_binding(binding_with_prompt("in-flight-new")).expect("replace mid-execute");
            assert!(
                !r#loop::is_binding_generation_current(old_gen),
                "replace must invalidate the snapshotted generation"
            );
        })
        .expect_err("in-flight execute must reject stale generation");
        assert_eq!(
            err.as_code(),
            "rejected_stale_generation",
            "Host must return a distinguishable generation-stale reject reason"
        );
        assert_eq!(r#loop::binding_state(), "bound");
        let new_gen = r#loop::query_binding().generation.expect("new gen");
        assert!(r#loop::is_binding_generation_current(new_gen));
        assert_ne!(old_gen, new_gen);

        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| {
                e.event == "onError" && e.category == Some("rejected_stale_generation")
            }),
            "onError(rejected_stale_generation) must be observable: {events:?}"
        );
        // Old Tools/Prompt must not win; a fresh execute under the new generation applies new prompt.
        let out = r#loop::execute_binding().expect("fresh execute under new gen");
        assert_eq!(out.applied_prompt, json!("in-flight-new"));
    });
}

#[test]
fn t1_query_and_present_do_not_authorize_execute() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        let _ = r#loop::query_binding();
        let _ = r#loop::present_ai_assistant_core().expect("Present");
        assert_eq!(r#loop::binding_state(), "unbound");
        let err = r#loop::execute_binding().expect_err("query/Present alone must not authorize");
        assert_eq!(err.as_code(), "rejected_unbound");
    });
}

#[test]
fn t2_reset_clears_current_session_id_to_none() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-reset-clear");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        assert_eq!(live_session_id().as_deref(), Some(old_sid.as_str()));

        r#loop::reset_binding().expect("Reset");
        assert_eq!(
            live_session_id(),
            None,
            "Reset must clear current_session_id to None (clear-first)"
        );
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t2_reset_old_session_not_reused_by_ensure_for_executable() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-reset-ensure");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        // Seed turns on the old session so reuse would be observable.
        let mut old = session::load_session(&old_sid).unwrap();
        old.turns.push(Turn {
            role: "user".into(),
            content: Some("old-turn".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&old).unwrap();

        r#loop::reset_binding().expect("Reset");
        assert_eq!(live_session_id(), None);

        // Re-bind then ensure: must mint a new session, not reuse the cut id.
        arm_plan_binding(&master);
        let ensured = r#loop::ensure_chat_session_core().expect("ensure after cut");
        let new_sid = ensured["session_id"].as_str().unwrap().to_string();
        assert_ne!(
            new_sid, old_sid,
            "ensure must not reuse a cut session id for executable chat"
        );
        assert_eq!(live_session_id().as_deref(), Some(new_sid.as_str()));
        let fresh = session::load_session(&new_sid).unwrap();
        assert!(
            fresh.turns.is_empty(),
            "new executable session must not carry old turns"
        );
    });
}

#[test]
fn t2_replace_set_clears_session_with_generation_advance() {
    with_sandbox(|| {
        let master_a = create_bound_plan("t2-replace-a");
        let master_b = create_bound_plan("t2-replace-b");
        arm_plan_binding(&master_a);
        let open = r#loop::open_ai_assistant_core(&master_a).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        assert_eq!(live_session_id().as_deref(), Some(old_sid.as_str()));

        arm_plan_binding(&master_b); // successful replace Set
        let new_gen = r#loop::query_binding().generation.expect("new gen");
        assert_ne!(old_gen, new_gen, "replace Set must advance generation");
        assert!(
            !r#loop::is_binding_generation_current(old_gen),
            "old generation must be invalid"
        );
        assert_eq!(
            live_session_id(),
            None,
            "replace Set must clear current_session_id (same semantics as Reset); \
             must not leave a new-generation + old-session window"
        );
    });
}

#[test]
fn t2_first_set_clears_pre_set_session_then_ensure_mints_new() {
    with_sandbox(|| {
        // unbound → ensure mints a session that must not auto-promote after Set.
        let pre = r#loop::ensure_chat_session_core().expect("ensure while unbound");
        let pre_sid = pre["session_id"].as_str().unwrap().to_string();
        assert_eq!(live_session_id().as_deref(), Some(pre_sid.as_str()));
        assert_eq!(r#loop::binding_state(), "unbound");

        let master = create_bound_plan("t2-first-set");
        arm_plan_binding(&master); // unbound→bound first successful Set
        assert_eq!(
            live_session_id(),
            None,
            "first successful Set must clear any pre-Set current_session_id"
        );
        assert_ne!(
            r#loop::query_binding().generation,
            None,
            "first Set must establish a live generation"
        );

        let ensured = r#loop::ensure_chat_session_core().expect("ensure after first Set");
        let new_sid = ensured["session_id"].as_str().unwrap().to_string();
        assert_ne!(
            new_sid, pre_sid,
            "unbound-era ensure session must not become the new binding executable context"
        );
    });
}

#[test]
fn t2_re_set_executable_chat_lands_on_new_session_without_old_turns() {
    with_sandbox(|| {
        let master_a = create_bound_plan("t2-re-set-a");
        let master_b = create_bound_plan("t2-re-set-b");
        let mock = spawn_scripted_llm(vec![
            assistant_text("旧绑定回复"),
            assistant_text("新绑定回复"),
        ]);
        install_llm_cfg(&mock);

        arm_plan_binding(&master_a);
        let open = r#loop::open_ai_assistant_core(&master_a).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let first = r#loop::agent_chat_turn_core(&old_sid, "你好", Some(&master_a)).unwrap();
        assert_eq!(first.body["terminal"], "none");
        let old_turns_len = session::load_session(&old_sid).unwrap().turns.len();
        assert!(old_turns_len >= 2, "old session should have turns");

        arm_plan_binding(&master_b); // replace Set → session cut
        assert_eq!(live_session_id(), None);

        let ensured = r#loop::ensure_chat_session_core().expect("ensure new");
        let new_sid = ensured["session_id"].as_str().unwrap().to_string();
        assert_ne!(new_sid, old_sid);
        let result = r#loop::agent_chat_turn_core(&new_sid, "继续", Some(&master_b)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap()
                .contains("新绑定回复")
        );
        let new_sess = session::load_session(&new_sid).unwrap();
        let blob = serde_json::to_string(&new_sess.turns).unwrap();
        assert!(
            !blob.contains("旧绑定回复") && !blob.contains("你好"),
            "new binding executable session must not carry old turns: {blob}"
        );
        // Old session file may still exist with its turns (cut ≠ delete).
        let old_still = session::load_session(&old_sid).unwrap();
        assert_eq!(old_still.turns.len(), old_turns_len);
    });
}

#[test]
fn t2_unbound_ensure_session_not_auto_promoted_on_set() {
    with_sandbox(|| {
        let pre = r#loop::ensure_chat_session_core().expect("unbound ensure");
        let pre_sid = pre["session_id"].as_str().unwrap().to_string();
        let mut seeded = session::load_session(&pre_sid).unwrap();
        seeded.turns.push(Turn {
            role: "user".into(),
            content: Some("unbound-era".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&seeded).unwrap();

        r#loop::set_binding(valid_binding()).expect("Set");
        assert_eq!(live_session_id(), None);

        let after = r#loop::ensure_chat_session_core().expect("ensure under new binding");
        let after_sid = after["session_id"].as_str().unwrap().to_string();
        assert_ne!(after_sid, pre_sid);
        let after_sess = session::load_session(&after_sid).unwrap();
        assert!(
            after_sess
                .turns
                .iter()
                .all(|t| t.content.as_deref() != Some("unbound-era")),
            "unbound-ensure session must not auto-promote into new binding context"
        );
    });
}

#[test]
fn t2_cut_does_not_wipe_turns_as_primary_means() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-no-wipe");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let mut sess = session::load_session(&old_sid).unwrap();
        sess.turns.push(Turn {
            role: "user".into(),
            content: Some("preserve-me".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&sess).unwrap();
        let turns_before = sess.turns.len();

        r#loop::reset_binding().expect("Reset");
        assert_eq!(live_session_id(), None);

        // Primary cut means clearing the live id — not wiping turns on the same id.
        let still = session::load_session(&old_sid).expect("disk json may remain");
        assert_eq!(
            still.turns.len(),
            turns_before,
            "cut must not use same-id turn wipe as the primary means"
        );
        assert!(
            still
                .turns
                .iter()
                .any(|t| t.content.as_deref() == Some("preserve-me"))
        );

        // Old id must not drive executable chat after cut.
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        install_llm_cfg(&mock);
        // Still unbound after Reset — re-bind so generation gate isn't the only rejector.
        arm_plan_binding(&master);
        assert_eq!(
            live_session_id(),
            None,
            "re-Set after Reset must also leave no live session (first/replace Set clears)"
        );
        let rejected = r#loop::agent_chat_turn_core(&old_sid, "drive-old", Some(&master));
        match rejected {
            Ok(result) => assert_ne!(result.body["terminal"], "none"),
            Err(_) => {}
        }
        assert_eq!(
            live_session_id(),
            None,
            "calling with cut id must not re-pin runtime to the cut session"
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 0);
    });
}

// --- T4: Host defensive cut (同语义 Reset; shell_close ≠ cut; 幂等) ---
// Must Close Before P2: DEFENSIVE_CUT_HOOK_PATH must be a stable, testable Host symbol.

#[test]
fn t4_defensive_cut_hook_path_is_confirmed_and_testable() {
    // Must Close Before: confirm hook-point file path is testable.
    assert_eq!(
        r#loop::DEFENSIVE_CUT_HOOK_PATH,
        "src-tauri/src/services/agent/loop/binding.rs::defensive_unbound",
        "P2 Done requires a confirmed, testable Host defensive-cut hook path"
    );
    // Explicit leave→Reset remains the primary path; defensive cut backs missed leave.
    assert_eq!(
        r#loop::DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN,
        [
            "frontend/src/todo-task/index.js::dispose",
            "frontend/src/todo-task/lifecycle.js::onTodosPageLeave",
            "frontend/src/todo-task/binding.js::resetTodosBinding",
            "src-tauri/src/services/agent/loop/binding.rs::reset_binding",
        ]
    );
    // Hook symbol is callable (not a UI-only stub).
    with_sandbox(|| {
        r#loop::defensive_unbound().expect("hook must be invokable");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t4_missed_reset_defensive_cut_matches_explicit_reset_semantics() {
    with_sandbox(|| {
        // Simulate「业务已退出仍 bound / 漏 Reset」: dispose leave signal missed,
        // Host still observes bound + live session → defensive_unbound.
        let master = create_bound_plan("t4-missed-reset");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let gen = r#loop::query_binding().generation.expect("gen");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(live_session_id().as_deref(), Some(old_sid.as_str()));
        r#loop::set_busy_for_tests(true);
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::defensive_unbound().expect("Host defensive cut on missed Reset");

        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(
            live_session_id(),
            None,
            "defensive cut must clear current_session_id (same as Reset)"
        );
        assert!(
            r#loop::query_binding().generation.is_none(),
            "defensive cut must invalidate generation"
        );
        assert!(
            !r#loop::is_binding_generation_current(gen),
            "old generation must not remain current"
        );
        assert!(
            r#loop::is_chat_cancelled_for_tests(),
            "defensive cut must cancel in-flight chat (symmetric with Reset)"
        );
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(
            event_names(&events),
            vec!["onUnbound"],
            "defensive cut must emit onUnbound like explicit Reset: {events:?}"
        );

        // Old binding/session must not drive executable dialogue.
        let exec_err = r#loop::execute_binding().expect_err("execute after defensive cut");
        assert_eq!(exec_err.as_code(), "rejected_unbound");
        // Release artificial busy (no live chat turn owns it); cut already cleared live session.
        r#loop::set_busy_for_tests(false);
        let chat = r#loop::agent_chat_turn_core(&old_sid, "漏Reset后不可执行", Some(&master))
            .expect("chat path returns body");
        assert_eq!(
            chat.body["terminal"], "business",
            "stale/cut session must reject without executable continuation"
        );
        assert_eq!(chat.body["wrote"], false);
    });
}

#[test]
fn t4_defensive_cut_after_explicit_reset_is_idempotent() {
    with_sandbox(|| {
        let master = create_bound_plan("t4-idempotent");
        arm_plan_binding(&master);
        let _ = r#loop::open_ai_assistant_core(&master).unwrap();
        r#loop::set_busy_for_tests(true);

        // Explicit Reset (dispose→onTodosPageLeave→resetTodosBinding→reset_binding) succeeded.
        r#loop::reset_binding().expect("explicit Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
        r#loop::clear_lifecycle_events_for_tests();

        // Defensive cut must be no-op / idempotent — no second onUnbound.
        r#loop::defensive_unbound().expect("defensive after Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
        assert!(r#loop::query_binding().generation.is_none());
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "idempotent defensive cut must not re-emit onUnbound: {events:?}"
        );

        r#loop::defensive_unbound().expect("second defensive still ok");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t4_defensive_cut_is_not_ui_only_weak_path() {
    with_sandbox(|| {
        // Forbidden: "只清 UI" — must truly unbound + invalidate gen + cut session + cancel.
        let master = create_bound_plan("t4-no-weak");
        arm_plan_binding(&master);
        let _ = r#loop::open_ai_assistant_core(&master).unwrap();
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::set_busy_for_tests(true);
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::defensive_unbound().expect("full cut");

        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(r#loop::current_binding_slot_count(), 0);
        assert!(!r#loop::is_binding_generation_current(gen));
        assert_eq!(live_session_id(), None);
        assert!(r#loop::is_chat_cancelled_for_tests());
        assert_eq!(
            event_names(&r#loop::drain_lifecycle_events()),
            vec!["onUnbound"]
        );
        // Defensive cut does not replace/omit the need for business explicit Reset —
        // after cut, explicit Reset remains valid (idempotent) and is still the primary path.
        r#loop::reset_binding().expect("explicit Reset still the primary path");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

// --- T5: shell sync (binding-changed) + distinguishable Host reject reasons ---

#[test]
fn t5_reset_cut_records_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "unbound"
            }),
            "Reset cut must record shell binding-changed → unbound: {syncs:?}"
        );
    });
}

#[test]
fn t5_defensive_unbound_records_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::defensive_unbound().expect("defensive cut");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "unbound"
            }),
            "defensive cut must record shell binding-changed (must not bypass shell sync): {syncs:?}"
        );
    });
}

#[test]
fn t5_successful_set_records_shell_binding_changed_bound() {
    with_sandbox(|| {
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::set_binding(valid_binding()).expect("Set");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "bound"
            }),
            "successful Set must record shell binding-changed → bound: {syncs:?}"
        );
    });
}

#[test]
fn t5_replace_set_records_shell_binding_changed_for_new_bound() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("v1")).expect("first Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::set_binding(binding_with_prompt("v2")).expect("replace Set");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "bound"
            }),
            "replace Set (new Bound) must record shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t5_idempotent_unbound_cut_does_not_repeat_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::reset_binding().expect("Reset");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::defensive_unbound().expect("idempotent defensive");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs
                .iter()
                .all(|e| e.event != "ai-assistant:binding-changed"),
            "idempotent already-unbound cut must not re-emit shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t5_host_reject_reasons_are_distinguishable() {
    with_sandbox(|| {
        // 1) unbound
        let unbound_err = r#loop::execute_binding().expect_err("unbound");
        assert_eq!(unbound_err.as_code(), "rejected_unbound");

        // 2) in-flight cancel (reset_cancelled)
        r#loop::set_binding(valid_binding()).expect("Set");
        let cancel_err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset mid-execute");
        })
        .expect_err("cancelled");
        assert_eq!(cancel_err.as_code(), "reset_cancelled");

        // 3) stale generation
        r#loop::set_binding(valid_binding()).expect("Set again");
        let stale_err = r#loop::execute_binding_during(|| {
            r#loop::set_binding(binding_with_prompt("replaced")).expect("replace");
        })
        .expect_err("stale");
        assert_eq!(stale_err.as_code(), "rejected_stale_generation");

        // 4) non-live session — distinguishable code on return body / signal
        let master = create_bound_plan("t5-reject-codes");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let live = open["session_id"].as_str().unwrap().to_string();
        let foreign = session::create_session(None, None).unwrap().session_id;
        assert_ne!(foreign, live);
        let rejected = r#loop::agent_chat_turn_core(&foreign, "ping", Some(&master))
            .expect("chat returns body with reject signal");
        let not_live_code = rejected.body["code"]
            .as_str()
            .expect("non-live reject must expose distinguishable code");
        assert_eq!(not_live_code, "rejected_not_live_session");

        // All four Host reject signals must be pairwise distinct.
        let codes = [
            unbound_err.as_code(),
            cancel_err.as_code(),
            stale_err.as_code(),
            not_live_code,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(
                    codes[i], codes[j],
                    "Host reject reasons must be distinguishable: {codes:?}"
                );
            }
        }
    });
}

#[test]
fn t5_not_live_session_reject_code_survives_after_cut() {
    with_sandbox(|| {
        let master = create_bound_plan("t5-cut-session-code");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        r#loop::reset_binding().expect("cut");
        let rejected = r#loop::agent_chat_turn_core(&old_sid, "after-cut", Some(&master))
            .expect("reject body");
        assert_eq!(rejected.body["code"], "rejected_not_live_session");
        assert_ne!(rejected.body["terminal"], "none");
        assert_eq!(rejected.body["wrote"], false);
    });
}


// --- T6: Todos leave still explicit Reset + layered acceptance L0/L1/L2 ---
// shell_close is NOT cut acceptance. Business ids must not enter Binding Contract.

#[test]
fn t6_layered_acceptance_markers_are_landed() {
    assert_eq!(
        r#loop::LAYERED_ACCEPTANCE_L0,
        [
            "unbound_reject_execute",
            "reset_idempotent",
            "mid_reset_cancel",
        ]
    );
    assert_eq!(
        r#loop::LAYERED_ACCEPTANCE_L1,
        [
            "old_session_not_executable_after_reset_or_replace_set",
            "stale_generation_reject_continue",
            "re_set_without_old_turns",
            "any_successful_set_clears_pre_set_session",
        ]
    );
    assert_eq!(
        r#loop::LAYERED_ACCEPTANCE_L2,
        ["missed_dispose_or_reset_defensive_cut_not_executable"]
    );
    // Explicit leave→Reset remains primary; defensive cut does not replace it.
    assert_eq!(
        r#loop::TODOS_EXPLICIT_LEAVE_RESET_PRIMARY,
        r#loop::DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN
    );
}

#[test]
fn t6_l0_unbound_reject_reset_idempotent_and_mid_reset_cancel() {
    with_sandbox(|| {
        // unbound reject
        let unbound_err = r#loop::execute_binding().expect_err("unbound must reject");
        assert_eq!(unbound_err.as_code(), "rejected_unbound");

        // Reset idempotent
        let master = create_bound_plan("t6-l0-idempotent");
        arm_plan_binding(&master);
        r#loop::reset_binding().expect("Reset");
        r#loop::reset_binding().expect("idempotent Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        let after = r#loop::execute_binding().expect_err("still unbound");
        assert_eq!(after.as_code(), "rejected_unbound");

        // mid-Reset cancel
        arm_plan_binding(&master);
        let cancel_err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset mid-execute");
        })
        .expect_err("mid-Reset must cancel execute");
        assert_eq!(cancel_err.as_code(), "reset_cancelled");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t6_l1_session_generation_and_re_set_cuts() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-l1-session");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let old_gen = r#loop::query_binding().generation.expect("gen");

        // Reset → old session not executable
        r#loop::reset_binding().expect("Reset");
        let rejected = r#loop::agent_chat_turn_core(&old_sid, "stale", Some(&master))
            .expect("reject body");
        assert_eq!(rejected.body["code"], "rejected_not_live_session");
        assert!(
            !r#loop::is_binding_generation_current(old_gen),
            "generation must be invalid after Reset"
        );

        // Any successful Set clears pre-set session
        let pre = r#loop::ensure_chat_session_core().expect("pre-set session");
        let pre_sid = pre["session_id"].as_str().unwrap().to_string();
        let master2 = create_bound_plan("t6-l1-reset2");
        arm_plan_binding(&master2);
        assert_eq!(
            live_session_id(),
            None,
            "successful Set must clear pre-set current_session_id"
        );
        assert_ne!(
            live_session_id().as_deref(),
            Some(pre_sid.as_str()),
            "pre-set session must not remain live after Set"
        );

        let open2 = r#loop::open_ai_assistant_core(&master2).unwrap();
        let sid_a = open2["session_id"].as_str().unwrap().to_string();
        // replace Set → re-Set without old session driving executable chat
        let master3 = create_bound_plan("t6-l1-replace");
        arm_plan_binding(&master3);
        assert_eq!(live_session_id(), None);
        let open3 = r#loop::open_ai_assistant_core(&master3).unwrap();
        let sid_b = open3["session_id"].as_str().unwrap().to_string();
        assert_ne!(sid_a, sid_b, "re-Set must mint a new session");
        let old_chat = r#loop::agent_chat_turn_core(&sid_a, "old-ctx", Some(&master3))
            .expect("reject old");
        assert_eq!(old_chat.body["code"], "rejected_not_live_session");
    });
}

#[test]
fn t6_l1_stale_generation_rejects_continue() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-l1-gen");
        arm_plan_binding(&master);
        let err = r#loop::execute_binding_during(|| {
            let other = create_bound_plan("t6-l1-gen-b");
            arm_plan_binding(&other);
        })
        .expect_err("stale generation must reject continue");
        assert_eq!(err.as_code(), "rejected_stale_generation");
    });
}

#[test]
fn t6_l2_missed_reset_defensive_cut_then_not_executable() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-l2-missed");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        // Missed dispose/Reset: Host defensive cut backs the leave signal.
        r#loop::defensive_unbound().expect("defensive cut");
        assert_eq!(r#loop::binding_state(), "unbound");
        let exec = r#loop::execute_binding().expect_err("not executable after L2 cut");
        assert_eq!(exec.as_code(), "rejected_unbound");
        let chat = r#loop::agent_chat_turn_core(&old_sid, "after-miss", Some(&master))
            .expect("reject");
        assert_eq!(chat.body["code"], "rejected_not_live_session");
    });
}

#[test]
fn t8_binding_request_helpers_route_create_turn_cancel_by_current_session_id() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let session_id = r#loop::ensure_chat_session_core()
            .expect("ensure")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session id")
            .to_string();
        assert_eq!(
            session::live_context_owner()
                .current_business_id()
                .as_deref(),
            Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)
        );
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));

        let create_calls = calls.clone();
        let turn_calls = calls.clone();
        let result = r#loop::ensure_create_then_turn(
            "hello",
            move |sid| {
                create_calls
                    .lock()
                    .unwrap()
                    .push(format!("create:{sid}"));
                Ok(())
            },
            move |sid, prompt| {
                turn_calls
                    .lock()
                    .unwrap()
                    .push(format!("turn:{sid}:{prompt}"));
                Ok(r#loop::ChatTurnResult {
                    body: json!({ "ok": true }),
                    emit_turn_completed: None,
                })
            },
        )
        .expect("ensure-create then turn");

        assert_eq!(result.body["ok"], true);
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                format!("create:{session_id}"),
                format!("turn:{session_id}:hello"),
            ]
        );
        assert!(
            !calls
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.contains(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)),
            "create/turn must record session identity, not Binding-derived identity"
        );

        let cancel_calls = calls.clone();
        r#loop::cancel_from_binding(move |sid| {
            cancel_calls
                .lock()
                .unwrap()
                .push(format!("cancel:{sid}"));
            Ok(())
        })
        .expect("cancel from Binding");
        assert_eq!(
            calls.lock().unwrap().last().cloned().as_deref(),
            Some(format!("cancel:{session_id}").as_str())
        );
        assert!(
            !calls
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.contains(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)),
            "cancel must record session identity, not Binding-derived identity"
        );
    });
}

#[test]
fn t8_binding_request_helpers_reject_unbound_without_session_fallback() {
    with_sandbox(|| {
        let create_called = Arc::new(Mutex::new(false));
        let cancel_called = Arc::new(Mutex::new(false));

        let create_called_for_cb = create_called.clone();
        let error = r#loop::ensure_create_then_turn(
            "must reject",
            move |_| {
                *create_called_for_cb.lock().unwrap() = true;
                Ok(())
            },
            |_, _| {
                panic!("turn must not run when Binding is unbound");
            },
        )
        .expect_err("unbound ensure-create must reject");
        assert_eq!(error, "rejected_unbound");
        assert!(!*create_called.lock().unwrap());

        let cancel_called_for_cb = cancel_called.clone();
        let error = r#loop::cancel_from_binding(move |_| {
            *cancel_called_for_cb.lock().unwrap() = true;
            Ok(())
        })
        .expect_err("unbound cancel must reject");
        assert_eq!(error, "rejected_unbound");
        assert!(!*cancel_called.lock().unwrap());
    });
}

#[test]
fn t4_reset_binding_with_close_routes_by_session_id_before_clearing_context() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let session_id = r#loop::ensure_chat_session_core()
            .expect("ensure")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session id")
            .to_string();
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let live_during_close = Arc::new(Mutex::new(
            None::<(String, Option<String>, Option<String>)>,
        ));
        let calls_for_close = calls.clone();
        let live_for_close = live_during_close.clone();

        r#loop::reset_binding_with_close(move |sid| {
            let live = session::live_context_owner();
            *live_for_close.lock().unwrap() = Some((
                r#loop::binding_state().to_string(),
                live.current_session_id(),
                live.current_business_id(),
            ));
            calls_for_close.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("reset and close");

        assert_eq!(*calls.lock().unwrap(), vec![session_id.clone()]);
        assert!(
            !calls.lock().unwrap().iter().any(|sid| sid == crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY),
            "close must receive session identity, not Binding-derived identity"
        );
        let during = live_during_close
            .lock()
            .unwrap()
            .clone()
            .expect("close must run");
        assert_eq!(
            during.0, "bound",
            "close must run before Binding is cleared"
        );
        assert_eq!(
            during.1.as_deref(),
            Some(session_id.as_str()),
            "close must run before live session is cleared"
        );
        assert_eq!(during.2.as_deref(), Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY));
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(session::live_context_owner().current_session_id().is_none());
        assert!(session::live_context_owner().current_business_id().is_none());
    });
}

#[test]
fn t_same_binding_switch_session_create_turn_do_not_reuse_prior_slot() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let (session_a, session_b, generation) =
            switch_session_without_resetting_binding("t2-switch-create-turn");
        assert_eq!(r#loop::query_binding().generation, Some(generation));

        let (calls, create, turn) = record_session_calls();
        r#loop::ensure_create_then_turn("hello-b", create, turn).expect("create/turn B");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                format!("create:{session_b}"),
                format!("turn:{session_b}:hello-b"),
            ]
        );
        assert!(
            !calls
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.contains(&session_a) || entry.contains(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)),
            "create/turn must use session B's slot, not session A or Binding identity"
        );
    });
}

#[test]
fn t_cancel_reset_only_hit_current_session_not_prior_binding_slot() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let (session_a, session_b, _) =
            switch_session_without_resetting_binding("t2-switch-cancel-reset");
        assert_eq!(
            session::live_context_owner()
                .current_session_id()
                .as_deref(),
            Some(session_b.as_str())
        );
        assert_ne!(
            session::live_context_owner()
                .current_session_id()
                .as_deref(),
            Some(session_a.as_str()),
            "live must not keep the prior session as current"
        );

        let cancel_calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let cancel_for_cb = cancel_calls.clone();
        r#loop::cancel_from_binding(move |sid| {
            cancel_for_cb.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("cancel current session");
        assert_eq!(*cancel_calls.lock().unwrap(), vec![session_b.clone()]);
        assert!(
            !cancel_calls
                .lock()
                .unwrap()
                .iter()
                .any(|sid| sid == &session_a || sid == crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY),
            "cancel must not hit the prior session or Binding identity"
        );

        let close_calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let close_for_cb = close_calls.clone();
        r#loop::reset_binding_with_close(move |sid| {
            close_for_cb.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("reset current session");
        assert_eq!(*close_calls.lock().unwrap(), vec![session_b]);
        assert!(
            !close_calls
                .lock()
                .unwrap()
                .iter()
                .any(|sid| sid == &session_a || sid == crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY),
            "reset must not close the prior Binding slot"
        );
    });
}

#[test]
fn t1_create_turn_cancel_reset_callbacks_receive_only_live_session_id() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let session_id = r#loop::ensure_chat_session_core()
            .expect("ensure")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session id")
            .to_string();
        let live = session::live_context_owner();
        assert_eq!(live.current_business_id().as_deref(), Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY));
        assert_eq!(live.current_session_id().as_deref(), Some(session_id.as_str()));

        let (calls, create, turn) = record_session_calls();
        r#loop::ensure_create_then_turn_with_error(
            "hello",
            "unbound".to_string(),
            "invalid".to_string(),
            "missing_session".to_string(),
            create,
            turn,
        )
        .expect("create/turn");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                format!("create:{session_id}"),
                format!("turn:{session_id}:hello"),
            ]
        );
        assert!(
            !calls
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.contains(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)),
            "create/turn must not receive Binding-derived identity"
        );

        let cancel_calls = calls.clone();
        r#loop::cancel_from_binding_with_error(
            "unbound".to_string(),
            "invalid".to_string(),
            move |sid| {
                cancel_calls.lock().unwrap().push(format!("cancel:{sid}"));
                Ok(())
            },
        )
        .expect("cancel");
        assert_eq!(
            calls.lock().unwrap().last().cloned().as_deref(),
            Some(format!("cancel:{session_id}").as_str())
        );

        let close_calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let close_for_cb = close_calls.clone();
        r#loop::reset_binding_with_close(move |sid| {
            close_for_cb.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("reset");
        assert_eq!(*close_calls.lock().unwrap(), vec![session_id.clone()]);
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(session::live_context_owner().current_session_id().is_none());
        assert!(session::live_context_owner().current_business_id().is_none());
    });
}

#[test]
fn t1_same_binding_switch_session_routes_to_new_slot() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let session_a = r#loop::ensure_chat_session_core()
            .expect("ensure A")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session A")
            .to_string();
        let master = create_bound_plan("t1-switch-session");
        let session_b = r#loop::open_ai_assistant_core(&master)
            .expect("open B")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session B")
            .to_string();
        assert_ne!(session_a, session_b);
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(
            session::live_context_owner()
                .current_business_id()
                .as_deref(),
            Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY),
            "switching session must not re-Set Binding"
        );

        let (calls, create, turn) = record_session_calls();
        r#loop::ensure_create_then_turn("hello-b", create, turn).expect("create/turn B");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                format!("create:{session_b}"),
                format!("turn:{session_b}:hello-b"),
            ]
        );
        assert!(
            !calls
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.contains(&session_a)),
            "create/turn must not reuse session A after the live id switched to B"
        );

        let cancel_calls = calls.clone();
        r#loop::cancel_from_binding(move |sid| {
            cancel_calls.lock().unwrap().push(format!("cancel:{sid}"));
            Ok(())
        })
        .expect("cancel B");
        assert_eq!(
            calls.lock().unwrap().last().cloned().as_deref(),
            Some(format!("cancel:{session_b}").as_str())
        );

        let close_calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let close_for_cb = close_calls.clone();
        r#loop::reset_binding_with_close(move |sid| {
            close_for_cb.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("reset B");
        assert_eq!(*close_calls.lock().unwrap(), vec![session_b]);
        assert!(!close_calls.lock().unwrap().iter().any(|sid| sid == &session_a));
    });
}

#[test]
fn t1_current_business_id_stays_on_live_but_is_not_isolation_key() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let session_id = r#loop::ensure_chat_session_core()
            .expect("ensure")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session id")
            .to_string();
        assert_eq!(
            session::live_context_owner()
                .current_business_id()
                .as_deref(),
            Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)
        );

        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_create = seen.clone();
        let seen_turn = seen.clone();
        r#loop::ensure_create_then_turn(
            "iso",
            move |sid| {
                seen_create.lock().unwrap().push(sid.to_string());
                Ok(())
            },
            move |sid, _prompt| {
                seen_turn.lock().unwrap().push(sid.to_string());
                Ok(r#loop::ChatTurnResult {
                    body: json!({ "ok": true }),
                    emit_turn_completed: None,
                })
            },
        )
        .expect("create/turn");
        let seen_cancel = seen.clone();
        r#loop::cancel_from_binding(move |sid| {
            seen_cancel.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("cancel");
        assert_eq!(*seen.lock().unwrap(), vec![session_id.clone(); 3]);
        assert_eq!(
            session::live_context_owner()
                .current_business_id()
                .as_deref(),
            Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY),
            "current_business_id remains on live after isolation routing"
        );
    });
}

#[test]
fn t1_binding_business_id_still_derives_from_binding_key() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let _ = r#loop::ensure_chat_session_core().expect("ensure");
        let binding = session::live_context_owner()
            .current_binding()
            .expect("bound");
        assert_eq!(
            session::binding_business_id(&binding).as_deref(),
            Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)
        );
        let q = r#loop::query_binding();
        assert_eq!(q.state, "bound");
        assert!(q.generation.is_some());
    });
}

#[test]
fn t1_public_wrappers_use_same_session_isolation_key_as_with_error() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        let session_id = r#loop::ensure_chat_session_core()
            .expect("ensure")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session id")
            .to_string();

        let public_seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let public_create = public_seen.clone();
        let public_turn = public_seen.clone();
        r#loop::ensure_create_then_turn(
            "wrap",
            move |sid| {
                public_create.lock().unwrap().push(sid.to_string());
                Ok(())
            },
            move |sid, prompt| {
                public_turn
                    .lock()
                    .unwrap()
                    .push(format!("{sid}:{prompt}"));
                Ok(r#loop::ChatTurnResult {
                    body: json!({ "ok": true }),
                    emit_turn_completed: None,
                })
            },
        )
        .expect("public wrapper");

        let typed_seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let typed_create = typed_seen.clone();
        let typed_turn = typed_seen.clone();
        r#loop::ensure_create_then_turn_with_error(
            "wrap",
            "unbound".to_string(),
            "invalid".to_string(),
            "missing_session".to_string(),
            move |sid| {
                typed_create.lock().unwrap().push(sid.to_string());
                Ok(())
            },
            move |sid, prompt| {
                typed_turn
                    .lock()
                    .unwrap()
                    .push(format!("{sid}:{prompt}"));
                Ok(r#loop::ChatTurnResult {
                    body: json!({ "ok": true }),
                    emit_turn_completed: None,
                })
            },
        )
        .expect("typed wrapper");

        assert_eq!(*public_seen.lock().unwrap(), *typed_seen.lock().unwrap());
        assert_eq!(
            *public_seen.lock().unwrap(),
            vec![session_id.clone(), format!("{session_id}:wrap")]
        );

        let public_cancel = Arc::new(Mutex::new(String::new()));
        let public_cancel_cb = public_cancel.clone();
        r#loop::cancel_from_binding(move |sid| {
            *public_cancel_cb.lock().unwrap() = sid.to_string();
            Ok(())
        })
        .expect("public cancel");

        let typed_cancel = Arc::new(Mutex::new(String::new()));
        let typed_cancel_cb = typed_cancel.clone();
        r#loop::cancel_from_binding_with_error(
            "unbound".to_string(),
            "invalid".to_string(),
            move |sid| {
                *typed_cancel_cb.lock().unwrap() = sid.to_string();
                Ok(())
            },
        )
        .expect("typed cancel");
        assert_eq!(*public_cancel.lock().unwrap(), session_id);
        assert_eq!(*public_cancel.lock().unwrap(), *typed_cancel.lock().unwrap());
    });
}

#[test]
fn t1_bound_without_live_session_rejects_create_turn_cancel() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY })).expect("Set");
        assert!(session::live_context_owner().current_session_id().is_none());
        assert!(session::live_context_owner().current_binding().is_some());
        assert_eq!(
            session::live_context_owner()
                .current_business_id()
                .as_deref(),
            Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY)
        );

        let create_called = Arc::new(Mutex::new(false));
        let create_flag = create_called.clone();
        let error = r#loop::ensure_create_then_turn_with_error(
            "must reject",
            "unbound".to_string(),
            "invalid".to_string(),
            "missing_session".to_string(),
            move |_| {
                *create_flag.lock().unwrap() = true;
                Ok(())
            },
            |_, _| -> Result<r#loop::ChatTurnResult, String> {
                panic!("turn must not run without live session_id")
            },
        )
        .expect_err("missing session must reject create/turn");
        assert_eq!(error, "missing_session");
        assert!(!*create_called.lock().unwrap());

        let public_error = r#loop::ensure_create_then_turn(
            "must reject",
            |_| panic!("public create must not run without live session_id"),
            |_, _| panic!("public turn must not run without live session_id"),
        )
        .expect_err("public wrapper missing session");
        assert_eq!(public_error, "rejected_not_live_session");

        let cancel_called = Arc::new(Mutex::new(false));
        let cancel_flag = cancel_called.clone();
        r#loop::cancel_from_binding(move |_| {
            *cancel_flag.lock().unwrap() = true;
            Ok(())
        })
        .expect_err("missing session must reject cancel");
        assert!(!*cancel_called.lock().unwrap());
        assert!(!r#loop::is_chat_cancelled_for_tests());
    });
}

#[test]
fn t6_explicit_reset_not_omitted_because_defensive_exists() {
    with_sandbox(|| {
        // Defensive cut exists, but normal leave still uses explicit Reset semantics.
        assert_eq!(
            r#loop::TODOS_EXPLICIT_LEAVE_RESET_PRIMARY.last().copied(),
            Some("src-tauri/src/services/agent/loop/binding.rs::reset_binding")
        );
        let master = create_bound_plan("t6-explicit-primary");
        arm_plan_binding(&master);
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("explicit Reset primary path");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onUnbound"),
            "explicit Reset must emit onUnbound: {events:?}"
        );
        // Defensive after explicit is idempotent — does not replace leave duty.
        r#loop::defensive_unbound().expect("idempotent defensive");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t6_binding_contract_rejects_top_level_business_ids() {
    with_sandbox(|| {
        // Business ids must not appear on Binding Contract query surface.
        r#loop::set_binding(valid_binding()).expect("Set without business top-level");
        let q = r#loop::query_binding();
        assert_query_is_business_agnostic(&q);
    });
}

#[test]
fn t2_key_only_set_loads_mcp_server_into_session_capability_context() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("legal key Set must succeed");
        assert_eq!(r#loop::binding_state(), "bound");

        let loaded = r#loop::loaded_mcp_server().expect("Set must load MCP Server config");
        let expected = mcp_registry::lookup(SEEDED_BUSINESS_KEY).expect("registry");
        assert_eq!(loaded.capability_description, expected.capability_description);
        assert_workbench_session_holds_live_ticket_seed_untouched();
        assert!(
            !loaded.capability_description.trim().is_empty(),
            "loaded config must be decision-level non-empty"
        );
    });
}

#[test]
fn t2_reset_binding_unloads_mcp_server_config() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        assert!(r#loop::loaded_mcp_server().is_some());

        r#loop::reset_binding().expect("Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(
            r#loop::loaded_mcp_server().is_none(),
            "Reset must unload MCP Server config from session capability context"
        );
    });
}

#[test]
fn t2_set_reset_public_json_reject_engine_selection_params() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        // Public Set must not accept engine selection parameters.
        for payload in [
            json!({ "key": SEEDED_BUSINESS_KEY, "engine": "host" }),
            json!({ "key": SEEDED_BUSINESS_KEY, "engine_type": "cursor" }),
            json!({ "key": SEEDED_BUSINESS_KEY, "engineType": "host" }),
        ] {
            let err = r#loop::try_set_binding_json(&payload)
                .expect_err("engine selection params must fail at public boundary");
            assert_eq!(err.as_code(), "set_invalid");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(r#loop::loaded_mcp_server().is_none());
        }
        // reset_binding takes no engine params (signature-level); idempotent ok.
        r#loop::reset_binding().expect("Reset");
    });
}

#[test]
fn t2_replace_set_with_new_key_replaces_loaded_mcp_config() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, McpServerConfig, SEEDED_BUSINESS_KEY};
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("first Set");
        let first = r#loop::loaded_mcp_server().expect("first loaded");

        mcp_registry::register(
            "alt_business_key",
            McpServerConfig {
                capability_description: "alternate mcp capability".into(),
                http_transport: mcp_registry::HttpMcpTransport {
                    name: "workbench".into(),
                    url: "http://127.0.0.1:9876/mcp".into(),
                    headers: Default::default(),
                },
            },
        )
        .expect("register alt");
        r#loop::try_set_binding_json(&key_only_payload("alt_business_key")).expect("replace Set");

        let second = r#loop::loaded_mcp_server().expect("replaced loaded");
        assert_ne!(first, second, "replace must not keep old MCP config alongside new");
        assert_eq!(second.capability_description, "alternate mcp capability");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::current_binding_slot_count(), 1);
    });
}

#[test]
fn t2_reset_when_unbound_is_idempotent_mcp_slot_stays_empty() {
    with_sandbox(|| {
        assert!(r#loop::loaded_mcp_server().is_none());
        r#loop::reset_binding().expect("Reset unbound");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());
        r#loop::reset_binding().expect("second Reset unbound");
        assert!(r#loop::loaded_mcp_server().is_none());
    });
}

#[test]
fn t2_unknown_key_fails_explicitly_without_destroying_prior_mcp_context() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("seed bound");
        let prior = r#loop::loaded_mcp_server().expect("prior MCP");
        let gen = r#loop::query_binding().generation;

        let err = r#loop::try_set_binding_json(&key_only_payload("unknown_business_key_xyz"))
            .expect_err("unknown key must fail");
        assert_eq!(err.as_code(), "unknown_key");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(
            r#loop::loaded_mcp_server().as_ref(),
            Some(&prior),
            "failed Set must not destroy prior session capability context"
        );
        assert_eq!(r#loop::query_binding().generation, gen);
    });
}

#[test]
fn t2_legacy_tools_prompt_callbacks_payload_rejected_at_public_boundary() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        // Unbound: legacy payload cannot bypass key→MCP lookup.
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "tool_a", "handle": "opaque-tool-a" }],
            "prompt": "opaque-system-prompt",
            "callbacks": {}
        }))
        .expect_err("legacy payload must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());

        // Bound via key: legacy payload still rejected; MCP context preserved.
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("key Set");
        let prior = r#loop::loaded_mcp_server().expect("loaded");
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "tool_a" }],
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("legacy payload must fail when bound");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::loaded_mcp_server().as_ref(), Some(&prior));
    });
}

#[test]
fn t2_missing_or_invalid_key_input_fails_explicitly() {
    with_sandbox(|| {
        for payload in [
            json!({}),
            json!({ "key": "" }),
            json!({ "key": "   " }),
            json!({ "key": null }),
            json!({ "master_task_id": "task_x" }),
            json!("todo_task"),
        ] {
            let err = r#loop::try_set_binding_json(&payload)
                .expect_err("missing/invalid key must fail");
            assert_eq!(err.as_code(), "set_invalid", "payload={payload}");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(r#loop::loaded_mcp_server().is_none());
        }
    });
}

#[test]
fn t2_binding_from_json_accepts_key_only_rejects_legacy() {
    with_sandbox(|| {
        use crate::services::agent::session;
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        let b = session::binding_from_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("key-only parse");
        assert_eq!(
            session::binding_business_key(&b).as_deref(),
            Some(SEEDED_BUSINESS_KEY),
            "parsed Binding must carry the business key"
        );

        let err = session::binding_from_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("legacy payload");
        assert_eq!(err.as_code(), "set_invalid");
    });
}

// --- t4: session capability context read-only consumption face ---
//
// Locks the Host Loop read API and keeps transport injection out of this slice.
//
// The same decision-level McpServerConfig shape is returned by this read face;
// field-level transport schema (stdio/http/…) remains deferred.
//
// A2 confirmed (not narrowed): Host mcp_registry is the sole lookup
// source; this face only exposes config already loaded by key-only Set from
// that authoritative table — it does not re-resolve or accept legacy payloads.

#[test]
fn t4_read_face_exposes_decision_level_shape_matching_registry_value() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");

        let view = r#loop::session_capability_mcp_config()
            .expect("bound session must expose loaded MCP config via read face");
        let expected = mcp_registry::lookup(SEEDED_BUSINESS_KEY).expect("registry");

        // Decision-level shape parity with t1 value (capability_description only).
        assert_eq!(view.capability_description, expected.capability_description);
        assert_workbench_session_holds_live_ticket_seed_untouched();
        assert!(
            !view.capability_description.trim().is_empty(),
            "read face must expose non-empty decision-level capability description"
        );
        // Shared decision-level form: same type/shape, no channel parameter.
        assert_eq!(
            view.capability_description,
            expected.capability_description,
            "Host Loop must consume the decision-level fields"
        );
    });
}

#[test]
fn t4_read_face_returns_none_when_unbound_or_after_reset() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        assert!(
            r#loop::session_capability_mcp_config().is_none(),
            "unbound consumption face must be empty/None"
        );

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        assert!(r#loop::session_capability_mcp_config().is_some());

        r#loop::reset_binding().expect("Reset");
        assert!(
            r#loop::session_capability_mcp_config().is_none(),
            "Reset must unload; read face must report removed"
        );
    });
}

#[test]
fn t4_read_face_is_readonly_consumer_mutate_does_not_rewrite_session() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let original = r#loop::session_capability_mcp_config().expect("loaded");

        // Consumer holds a detached view; mutating it must not rewrite session state.
        let mut local = r#loop::session_capability_mcp_config().expect("clone view");
        local.capability_description = "mutated-by-consumer".into();
        assert_ne!(local.capability_description, original.capability_description);

        let reread = r#loop::session_capability_mcp_config().expect("still loaded");
        assert_eq!(
            reread, original,
            "read face must not provide a business/external write path into session config"
        );
    });
}

#[test]
fn t4_only_set_reset_lifecycle_may_change_loaded_config() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, McpServerConfig, SEEDED_BUSINESS_KEY};
        assert!(r#loop::session_capability_mcp_config().is_none());

        // Lifecycle Set loads.
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let first = r#loop::session_capability_mcp_config().expect("after Set");

        // Registry mutation alone must not rewrite the already-loaded session view
        // (A2: table is lookup source at Set time; read face does not re-inject).
        mcp_registry::register(
            SEEDED_BUSINESS_KEY,
            McpServerConfig {
                capability_description: "registry-mutated-after-set".into(),
                http_transport: mcp_registry::HttpMcpTransport {
                    name: "workbench".into(),
                    url: "http://127.0.0.1:9876/mcp".into(),
                    headers: Default::default(),
                },
            },
        )
        .expect("register overwrite");
        let still = r#loop::session_capability_mcp_config().expect("unchanged without Set");
        assert_eq!(
            still, first,
            "read face must not re-lookup/re-inject; only Set/Reset lifecycle may change"
        );

        // Lifecycle replace Set updates.
        mcp_registry::register(
            "alt_for_t4",
            McpServerConfig {
                capability_description: "alt capability for t4".into(),
                http_transport: mcp_registry::HttpMcpTransport {
                    name: "workbench".into(),
                    url: "http://127.0.0.1:9876/mcp".into(),
                    headers: Default::default(),
                },
            },
        )
        .expect("alt");
        r#loop::try_set_binding_json(&key_only_payload("alt_for_t4")).expect("replace Set");
        let second = r#loop::session_capability_mcp_config().expect("after replace");
        assert_ne!(second, first);
        assert_eq!(second.capability_description, "alt capability for t4");

        // Lifecycle Reset clears.
        r#loop::reset_binding().expect("Reset");
        assert!(r#loop::session_capability_mcp_config().is_none());
    });
}

#[test]
fn t4_read_face_cannot_reinject_legacy_tools_prompt_callbacks() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let via_read = r#loop::session_capability_mcp_config().expect("read face");

        // Consumption face returns decision-level MCP config only — not a Binding
        // write path. Legacy tools/prompt/callbacks cannot be pushed back through it.
        assert!(
            !via_read.capability_description.contains("\"tools\""),
            "read face must not surface legacy tools payload shape"
        );
        // Public Set still rejects legacy payload; session view unchanged.
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "tool_a", "handle": "opaque" }],
            "prompt": via_read.capability_description,
            "callbacks": {}
        }))
        .expect_err("legacy reinject via Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(
            r#loop::session_capability_mcp_config().as_ref(),
            Some(&via_read),
            "failed legacy reinject must leave read-face config unchanged"
        );
    });
}

#[test]
fn t4_a1_a2_handoff_assumptions_confirmed_not_narrowed() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        // A1: one decision-level shape, with no channel-specific fields.
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let face = r#loop::session_capability_mcp_config().expect("face");
        let table = mcp_registry::lookup(SEEDED_BUSINESS_KEY).expect("table");
        assert_eq!(
            face.capability_description, table.capability_description,
            "A1 confirmed: read face exposes the registry decision-level form"
        );
        assert_workbench_session_holds_live_ticket_seed_untouched();

        // A2: Host authoritative table is the sole lookup source at Set; Binding
        // callers only need the stable business key (already exercised by Set path).
        assert_eq!(
            face.capability_description, table.capability_description,
            "A2 confirmed: loaded view originates from Host registry lookup, not caller payload"
        );
        assert!(
            r#loop::SESSION_CAPABILITY_READ_FACE_REGISTRY_SHAPE,
            "A1 must be explicitly confirmed in delivery"
        );
        assert!(
            r#loop::SESSION_CAPABILITY_READ_FACE_A2_HOST_REGISTRY_SOLE_LOOKUP,
            "A2 must be explicitly confirmed in delivery (not silently narrowed)"
        );
    });
}

#[test]
fn t4_workbench_key_only_set_binds_seeded_workbench_mcp() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("try_set_binding_json({{ key: workbench }}) must succeed when seeded");
        assert_bound_key(SEEDED_BUSINESS_KEY);
        assert_eq!(SEEDED_BUSINESS_KEY, "workbench");
        let loaded = r#loop::loaded_mcp_server().expect("workbench mcp");
        let expected = mcp_registry::lookup(SEEDED_BUSINESS_KEY).expect("registry workbench");
        assert_eq!(loaded.capability_description, expected.capability_description);
        assert_workbench_session_holds_live_ticket_seed_untouched();
        assert!(
            loaded.http_transport().url.ends_with("/mcp/workbench"),
            "workbench seed URL must be /mcp/workbench"
        );
    });
}

#[test]
fn t4_old_app_keys_notes_and_todo_task_fail_set_binding() {
    with_sandbox(|| {
        for key in ["notes", "todo_task"] {
            let err = r#loop::try_set_binding_json(&key_only_payload(key))
                .expect_err("old App key must fail Host Binding");
            assert_eq!(err.as_code(), "unknown_key", "key={key}");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(r#loop::loaded_mcp_server().is_none());
        }
    });
}

#[test]
fn t4_only_other_binding_page_resets_notes() {
    let routes = repo_file("frontend/src/app-shell/routes.ts");
    let mount_plan = function_slice(&routes, "function mountTodoTasksRoute");
    assert!(
        !mount_plan.contains("resetNotesBinding")
            && !mount_plan.contains("set_binding")
            && !mount_plan.contains("reset_binding"),
        "mountTodoTasksRoute must not Set/Reset Binding"
    );
    let mount_wb = function_slice(&routes, "function mountWorkbench");
    assert!(
        !mount_wb.contains("buildNotesBinding")
            && !mount_wb.contains("set_binding")
            && !mount_wb.contains("reset_binding"),
        "mountWorkbench must not Set/Reset Binding"
    );
    let mount_home = function_slice(&routes, "function mountHomeRoute");
    assert!(
        !mount_home.contains("resetNotesBinding") && !mount_home.contains("reset_binding"),
        "home / Hub host route must not Reset Binding"
    );
}

#[test]
fn t4_old_app_keys_still_unknown_after_seed_defaults() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        mcp_registry::clear_for_tests();
        for key in ["notes", "todo_task"] {
            let err = r#loop::try_set_binding_json(&key_only_payload(key))
                .expect_err("cleared old key must fail Set");
            assert_eq!(err.as_code(), "unknown_key");
        }
        assert_eq!(r#loop::binding_state(), "unbound");

        mcp_registry::seed_defaults();
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("seeded workbench Set");
        for key in ["notes", "todo_task"] {
            let err = r#loop::try_set_binding_json(&key_only_payload(key))
                .expect_err("old App key still hard-reject after seed");
            assert_eq!(err.as_code(), "unknown_key", "key={key}");
        }
        assert_bound_key(SEEDED_BUSINESS_KEY);
    });
}

#[test]
fn t4_empty_key_is_invalid_and_does_not_fall_to_workbench() {
    with_sandbox(|| {
        use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
        let err = r#loop::try_set_binding_json(&key_only_payload(""))
            .expect_err("empty key must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("workbench Set");
        let err = r#loop::try_set_binding_json(&key_only_payload(""))
            .expect_err("empty key must not replace workbench");
        assert_eq!(err.as_code(), "set_invalid");
        assert_bound_key(SEEDED_BUSINESS_KEY);
    });
}

#[test]
fn t4_cursor_ide_is_not_an_app_binding_key() {
    with_sandbox(|| {
        let err = r#loop::try_set_binding_json(&key_only_payload("cursor_ide"))
            .expect_err("cursor_ide must not become the App Binding key");
        assert_eq!(err.as_code(), "unknown_key");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t4_notes_binding_consumer_follows_todos_key_only_contract() {
    let binding = repo_file("frontend/src/todo-task/commands/binding.ts");
    let index = repo_file("frontend/src/todo-task/index.ts");
    assert!(
        binding.contains("WORKBENCH_BUSINESS_KEY") && binding.contains("'workbench'"),
        "binding.ts must export WORKBENCH_BUSINESS_KEY = workbench"
    );
    assert!(
        binding.contains("export async function setWorkbenchBinding"),
        "setWorkbenchBinding must exist (key-only workbench Set)"
    );
    assert!(
        !binding.contains("NOTES_BUSINESS_KEY")
            && !binding.contains("TODOS_BUSINESS_KEY")
            && !binding.contains("assembleNotesBindingBody")
            && !binding.contains("buildNotesBinding")
            && !binding.contains("resetNotesBinding"),
        "old notes/todos Binding helpers must be deleted"
    );
    let set_fn = function_slice(&binding, "async function setWorkbenchBinding");
    assert!(
        set_fn.contains("set_binding") && set_fn.contains("key"),
        "setWorkbenchBinding must invoke key-only set_binding"
    );
    assert!(
        !set_fn.contains("tools:") && !set_fn.contains("prompt:") && !set_fn.contains("callbacks:"),
        "workbench Set must not assemble tools/prompt/callbacks"
    );
    assert!(
        !binding.contains("engine_type") && !binding.contains("engineType"),
        "Workbench Binding must not select an engine"
    );
    assert!(
        !index.contains("assembleNotesBindingBody")
            && !index.contains("buildNotesBinding")
            && !index.contains("resetNotesBinding")
            && !index.contains("NOTES_BUSINESS_KEY"),
        "todo-task/index.ts must not re-export old Notes Binding symbols"
    );
}

/// P4: Binding Set workbench must use `try_set_binding_json({ key: "workbench" })`.
/// typed `set_binding(Binding)` does not do registry lookup and cannot substitute.
#[test]
fn t6_p4_try_set_binding_json_workbench_succeeds() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect("P4: try_set_binding_json({ key: workbench }) must succeed");
        assert_bound_key("workbench");
    });
}

#[test]
fn t6_p4_typed_set_binding_does_not_substitute_for_workbench_key_lookup() {
    with_sandbox(|| {
        r#loop::set_binding(empty_tools_binding()).expect("typed Set");
        assert_eq!(r#loop::binding_state(), "bound");
        assert!(
            r#loop::loaded_mcp_server().is_none(),
            "typed set_binding(Binding) must not registry-lookup workbench"
        );
        assert_ne!(
            session::live_context_owner().current_business_id().as_deref(),
            Some("workbench"),
            "typed set_binding must not bind the workbench business key"
        );
    });
}

// --- t5: Binding Set injects workbench ticket; Keychain failure is set_invalid ---

#[test]
fn t5_workbench_set_issues_new_ticket_when_slot_empty() {
    with_sandbox(|| {
        reset_workbench_slot();
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect("workbench Set must succeed");
        assert_eq!(r#loop::binding_state(), "bound");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onBound"),
            "successful Set must emit onBound: {events:?}"
        );
        assert_workbench_session_holds_live_ticket_seed_untouched();
    });
}

#[test]
fn t5_workbench_set_reuses_live_ticket_and_does_not_issue_a_second() {
    with_sandbox(|| {
        reset_workbench_slot();
        let existing = issue_for_slot(Slot::Workbench).expect("pre-issue live");
        r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect("workbench Set");
        assert_workbench_session_holds_live_ticket_seed_untouched();
        let auth = session_authorization_bearer().expect("session auth");
        let session_handle =
            TicketHandle::from_secret(auth.strip_prefix("Bearer ").expect("Bearer"));
        assert_eq!(session_handle, existing);
        let reused = issue_for_slot(Slot::Workbench).expect("reuse after Set");
        assert_eq!(reused, existing);
    });
}

#[test]
fn t5_registry_seed_stays_without_authorization_after_set() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set");
        let seed = mcp_registry::lookup("workbench").expect("seed");
        assert!(
            !seed.http_transport.headers.contains_key("Authorization"),
            "seeded_http_transport / lookup(workbench) must stay without ticket header"
        );
        assert!(
            session_authorization_bearer()
                .as_deref()
                .is_some_and(|header| header.starts_with("Bearer ")),
            "session transport copy must hold Authorization after Set"
        );
    });
}

#[test]
fn t5_unknown_key_is_not_used_for_keychain_failure() {
    with_sandbox(|| {
        reset_workbench_slot();
        test_force_keychain_unavailable(true);
        let err = r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect_err("Keychain failure must fail Set");
        assert_eq!(err.as_code(), "set_invalid");
        assert_ne!(err.as_code(), "unknown_key");
        test_force_keychain_unavailable(false);
        let missing = r#loop::try_set_binding_json(&json!({ "key": "no_such_binding_key" }))
            .expect_err("missing key");
        assert_eq!(missing.as_code(), "unknown_key");
    });
}

#[test]
fn t5_set_error_codes_remain_set_invalid_and_unknown_key_only() {
    let src = crate::test_support::read_rs_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/agent/session"
    ));
    assert!(src.contains("pub fn set_invalid"));
    assert!(src.contains("pub fn unknown_key"));
    assert!(
        !src.contains("fn keychain") && !src.contains("SetError::ticket"),
        "SetError must not gain Keychain/ticket codes"
    );
}

#[test]
fn t5_keychain_write_failure_keeps_unbound_issues_no_ticket_and_skips_on_bound() {
    with_sandbox(|| {
        reset_workbench_slot();
        r#loop::clear_lifecycle_events_for_tests();
        test_force_keychain_unavailable(true);
        let err = r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect_err("Keychain write failure must fail Set");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());
        assert!(session_authorization_bearer().is_none());
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onBound"),
            "Keychain failure must not emit onBound: {events:?}"
        );
        test_force_keychain_unavailable(false);
        let record = ledger_record(Slot::Workbench).expect("ledger after failed Set");
        assert!(
            record
                .as_ref()
                .map(|row| row.state != TicketState::Live)
                .unwrap_or(true),
            "Keychain failure must not leave a Live workbench ticket"
        );
    });
}

#[test]
fn t5_keychain_write_failure_keeps_original_binding() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("original typed Binding");
        let gen = r#loop::query_binding().generation;
        r#loop::clear_lifecycle_events_for_tests();
        reset_workbench_slot();
        test_force_keychain_unavailable(true);
        let err = r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect_err("Keychain failure");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::query_binding().generation, gen);
        assert!(
            r#loop::loaded_mcp_server().is_none(),
            "failed workbench Set must not load a ticketed transport"
        );
        assert!(session_authorization_bearer().is_none());
        let events = r#loop::drain_lifecycle_events();
        assert!(events.iter().all(|e| e.event != "onBound"));
        test_force_keychain_unavailable(false);
        let record = ledger_record(Slot::Workbench).expect("ledger");
        assert!(
            record
                .as_ref()
                .map(|row| row.state != TicketState::Live)
                .unwrap_or(true),
            "failed Set must not issue a ticket"
        );
    });
}

#[test]
fn t5_set_path_must_not_log_ticket_or_full_authorization() {
    with_sandbox(|| {
        reset_workbench_slot();
        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set");
        let auth = session_authorization_bearer().expect("auth");
        let secret = auth
            .strip_prefix("Bearer ")
            .expect("Bearer")
            .to_string();
        let loop_src = LOOP_SRC;
        assert!(
            loop_src.contains("issue_for_slot"),
            "try_set_binding_json must call issue_for_slot after workbench lookup"
        );
        let debug = loop_src
            .find("[DEBUG-binding-transition]")
            .map(|idx| &loop_src[idx..idx.saturating_add(240)])
            .unwrap_or("");
        assert!(
            !debug.contains("Authorization") && !debug.contains("headers"),
            "binding-transition debug must not print ticket headers"
        );
        let events = format!("{:?}", r#loop::drain_lifecycle_events());
        assert_secret_absent_from(&events, &secret);
        assert_secret_absent_from(&format!("{:?}", r#loop::SetError::set_invalid()), &secret);
        assert_secret_absent_from(r#loop::SetError::set_invalid().as_code(), &secret);
    });
}

#[test]
fn t6_reset_after_workbench_set_unloads_session_ticket_keeps_ledger_live() {
    with_sandbox(|| {
        assert!(
            r#loop::RESET_UNLOADS_SESSION_ONLY,
            "Reset must lock session-unload-only"
        );
        reset_workbench_slot();
        r#loop::try_set_binding_json(&json!({ "key": "workbench" }))
            .expect("workbench Set must inject a ticket");
        assert_workbench_session_holds_live_ticket_seed_untouched();
        let issued = session_ticket_handle();
        let before = live_workbench_record();
        assert_eq!(before.state, TicketState::Live);
        assert_eq!(before.handle, issued);

        r#loop::reset_binding().expect("Reset");
        assert_session_unloaded_without_ticket_header();

        let after = live_workbench_record();
        assert_eq!(after.state, TicketState::Live);
        assert_eq!(after.handle, issued);
        assert_eq!(after.handle, before.handle);
    });
}

#[test]
fn t6_reset_then_set_reuses_same_workbench_handle() {
    with_sandbox(|| {
        reset_workbench_slot();
        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("first Set");
        let first = session_ticket_handle();
        r#loop::reset_binding().expect("Reset");
        assert_session_unloaded_without_ticket_header();
        assert_eq!(live_workbench_record().state, TicketState::Live);
        assert_eq!(live_workbench_record().handle, first);

        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("next Set");
        let second = session_ticket_handle();
        assert_eq!(second, first);
        let reused = issue_for_slot(Slot::Workbench).expect("reuse Live ticket");
        assert_eq!(reused, first);
        assert_eq!(live_workbench_record().state, TicketState::Live);
        assert_eq!(live_workbench_record().handle, first);
        assert_workbench_session_holds_live_ticket_seed_untouched();
    });
}

#[test]
fn t6_reset_binding_and_with_close_must_not_call_revoke_for_slot() {
    let src = LOOP_SRC;
    let reset_fn = rust_pub_item(src, "pub fn reset_binding()");
    let close_fn = rust_pub_item(src, "pub fn reset_binding_with_close");
    assert!(
        !reset_fn.contains("revoke_for_slot"),
        "reset_binding MUST NOT call revoke_for_slot"
    );
    assert!(
        !close_fn.contains("revoke_for_slot"),
        "reset_binding_with_close MUST NOT call revoke_for_slot"
    );
    assert!(
        r#loop::RESET_UNLOADS_SESSION_ONLY,
        "Reset contract marker must stay true"
    );
}

#[test]
fn t6_reset_binding_with_close_does_not_revoke_workbench_ledger() {
    with_sandbox(|| {
        reset_workbench_slot();
        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set");
        let issued = session_ticket_handle();
        let session_id = r#loop::ensure_chat_session_core()
            .expect("ensure")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("session id")
            .to_string();
        let closed = Arc::new(Mutex::new(Vec::<String>::new()));
        let closed_for_fn = closed.clone();
        r#loop::reset_binding_with_close(move |sid| {
            closed_for_fn.lock().unwrap().push(sid.to_string());
            Ok(())
        })
        .expect("reset_binding_with_close");
        assert_eq!(*closed.lock().unwrap(), vec![session_id]);
        assert_session_unloaded_without_ticket_header();
        let record = live_workbench_record();
        assert_eq!(record.state, TicketState::Live);
        assert_eq!(record.handle, issued);
    });
}

#[test]
fn t6_already_unbound_reset_is_idempotent_and_does_not_change_ledger() {
    with_sandbox(|| {
        reset_workbench_slot();
        let empty = ledger_record(Slot::Workbench).expect("ledger before any Set");
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::reset_binding().expect("Reset while never bound");
        r#loop::reset_binding().expect("second Reset while never bound");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());
        assert_eq!(
            ledger_record(Slot::Workbench).expect("ledger after unbound Reset"),
            empty
        );

        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set");
        let issued = session_ticket_handle();
        r#loop::reset_binding().expect("Reset after Set");
        let after_first = live_workbench_record();
        assert_eq!(after_first.state, TicketState::Live);
        assert_eq!(after_first.handle, issued);
        assert_session_unloaded_without_ticket_header();

        r#loop::reset_binding().expect("already-unbound Reset");
        r#loop::reset_binding().expect("second already-unbound Reset");
        assert_session_unloaded_without_ticket_header();
        let after = live_workbench_record();
        assert_eq!(after.state, TicketState::Live);
        assert_eq!(after.handle, issued);
        assert_eq!(after.handle, after_first.handle);
    });
}

#[test]
fn t6_reset_path_must_not_log_ticket_or_full_authorization() {
    with_sandbox(|| {
        reset_workbench_slot();
        r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set");
        let secret = session_authorization_bearer()
            .expect("session ticket header")
            .strip_prefix("Bearer ")
            .expect("Bearer")
            .to_string();
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let src = LOOP_SRC;
        let reset_fn = rust_pub_item(src, "pub fn reset_binding()");
        let close_fn = rust_pub_item(src, "pub fn reset_binding_with_close");
        let debug = reset_fn
            .find("[DEBUG-binding-transition]")
            .map(|idx| &reset_fn[idx..idx.saturating_add(240)])
            .unwrap_or("");
        assert!(
            !debug.contains("Authorization") && !debug.contains("headers"),
            "Reset debug must not print ticket headers"
        );
        assert!(
            !reset_fn.contains("revoke_for_slot") && !close_fn.contains("revoke_for_slot"),
            "Reset path must not call revoke_for_slot"
        );
        let events = format!("{:?}", r#loop::drain_lifecycle_events());
        assert_secret_absent_from(&events, &secret);
        assert_secret_absent_from(&format!("{:?}", r#loop::SetError::set_invalid()), &secret);
        assert_secret_absent_from(r#loop::SetError::set_invalid().as_code(), &secret);
        assert_secret_absent_from(debug, &secret);
    });
}
