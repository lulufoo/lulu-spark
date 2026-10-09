//! LW-180: `run_loop` stamps each Step's clock where its own work happens.

use super::support::*;

fn clock_of(step: &session::Step) -> (i64, i64, i64) {
    (
        step.clock.create_ts.expect("create_ts"),
        step.clock.start_ts.expect("start_ts"),
        step.clock.end_ts.expect("end_ts"),
    )
}

#[test]
fn text_reply_stamps_user_as_a_point_and_assistant_as_the_model_request_span() {
    with_sandbox(|| {
        let mock = spawn_scripted_llm(vec![assistant_text("你好")]);
        let master = create_bound_plan("主计划");
        let mut session = session::create_session().expect("session");
        arm_plan_binding(&master);
        r#loop::run_loop(&mut session, "你好", &cfg_for(&mock));

        let (u_create, u_start, u_end) = clock_of(&session.steps[0]);
        assert_eq!((u_create, u_start), (u_start, u_end), "user is one point");
        let (a_create, a_start, a_end) = clock_of(&session.steps[1]);
        assert!(a_start >= u_end, "the request starts after the user sent");
        assert!(a_end >= a_start);
        assert!(a_create >= a_end, "pushed after the response returned");

        let reloaded = session::load_session(&session.session_id).expect("reload");
        assert_eq!(reloaded.steps[0].clock, session.steps[0].clock);
        assert_eq!(reloaded.steps[1].clock, session.steps[1].clock);
    });
}

#[test]
fn tool_turn_gives_each_row_its_own_window_in_order() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    let notes = sandbox.data_dir().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("notes raw");
    fs::create_dir_all(notes.join("digest")).expect("notes digest");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("notes index");
    let (mcp, mcp_port) = start_isolated_mcp(&sandbox);
    register_test_mcp("spark", mcp_port);

    let call = json!([{
        "id": "selection_1", "type": "function",
        "function": { "name": "get_all_notes_catalog", "arguments": "{}" }
    }]);
    let mock = spawn_scripted_llm(vec![assistant_tools(call, None), assistant_text("好了")]);
    r#loop::try_set_binding_json(&json!({ "key": "spark" })).expect("Set spark binding");
    let mut session = session::create_session().expect("session");
    r#loop::run_loop(&mut session, "看看目录", &cfg_for(&mock));
    stop_embedded_mcp_runtime(mcp).expect("stop MCP");

    let roles: Vec<&str> = session.steps.iter().map(|s| s.role.as_str()).collect();
    assert_eq!(roles, ["user", "assistant", "tool", "assistant"]);
    let user = clock_of(&session.steps[0]);
    let tool_call = clock_of(&session.steps[1]);
    let tool_result = clock_of(&session.steps[2]);
    let final_reply = clock_of(&session.steps[3]);

    // First-tool latency is the first tool_result's start_ts minus the user's end_ts.
    assert!(tool_call.1 >= user.2, "model request starts after the user sent");
    assert!(tool_call.2 >= tool_call.1);
    assert!(tool_result.1 >= tool_call.2, "invoke starts after the tool_call response");
    assert!(tool_result.2 >= tool_result.1);
    assert!(final_reply.1 >= tool_result.2, "follow-up request starts after the tool returned");
    assert!(final_reply.2 >= final_reply.1);
    assert!(tool_result.0 >= tool_result.2, "tool_result is pushed after invoke returned");

    let reloaded = session::load_session(&session.session_id).expect("reload");
    for (stored, live) in reloaded.steps.iter().zip(&session.steps) {
        assert_eq!(stored.clock, live.clock, "persisted clock equals the live clock");
    }
}

#[test]
fn host_synthesized_reply_is_a_point_while_a_failed_request_keeps_its_span() {
    with_sandbox(|| {
        // The Binding is unbound, so the Host writes the reply itself: no model request.
        let mut unbound = session::create_session().expect("session");
        r#loop::run_loop(&mut unbound, "你好", &cfg_for(&spawn_scripted_llm(vec![])));
        let host_reply = clock_of(unbound.steps.last().expect("host reply"));
        assert_eq!(host_reply.1, host_reply.2, "Host reply has no work window");
        assert_eq!(host_reply.0, host_reply.1);

        // A rejected response still cost a request: the error row spans it.
        let rejected = (
            200,
            json!({
                "model": "glm-reply",
                "choices": [{ "finish_reason": "sensitive",
                              "message": { "role": "assistant", "content": "x" } }]
            }),
        );
        let mock = spawn_scripted_llm(vec![rejected]);
        let master = create_bound_plan("主计划");
        let mut session = session::create_session().expect("session");
        arm_plan_binding(&master);
        r#loop::run_loop(&mut session, "你好", &cfg_for(&mock));
        let user = clock_of(&session.steps[0]);
        let error_row = clock_of(session.steps.last().expect("error row"));
        assert!(error_row.1 >= user.2);
        assert!(error_row.2 >= error_row.1);
    });
}
