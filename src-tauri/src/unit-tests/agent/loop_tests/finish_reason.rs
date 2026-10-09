//! LW-171: the loop words each abnormal response differently and keeps what arrived.

use super::support::*;

fn response(finish: &str, content: &str, thinking: Option<&str>) -> (u16, Value) {
    let mut message = json!({ "role": "assistant", "content": content });
    if let Some(text) = thinking {
        message["reasoning_content"] = json!(text);
    }
    (
        200,
        json!({
            "model": "glm-reply",
            "choices": [{ "finish_reason": finish, "message": message }]
        }),
    )
}

/// One Host turn against a scripted reply; returns the outcome and the session.
fn run_one(reply: (u16, Value)) -> (TurnOutcome, session::Session) {
    let mock = spawn_scripted_llm(vec![reply]);
    let master = create_bound_plan("主计划");
    let mut session = session::create_session().expect("session");
    arm_plan_binding(&master);
    let outcome = r#loop::run_loop(&mut session, "你好", &cfg_for(&mock));
    (outcome, session)
}

#[test]
fn thinking_that_used_up_the_budget_has_its_own_message_and_is_kept() {
    with_sandbox(|| {
        let (outcome, session) = run_one(response("length", "", Some("想了很久")));
        assert!(matches!(outcome.terminal, Terminal::Error));
        assert!(outcome.reply_text.contains("思考占满"), "{}", outcome.reply_text);
        let last = session.steps.last().expect("error step");
        assert_eq!(last.content.as_deref(), Some(outcome.reply_text.as_str()));
        assert_eq!(last.reasoning_content.as_deref(), Some("想了很久"));
        assert_eq!(last.finish_reason.as_deref(), Some("length"));
        assert_eq!(last.model.as_deref(), Some("glm-reply"));
    });
}

#[test]
fn length_with_text_stays_the_plain_truncated_message() {
    with_sandbox(|| {
        let (outcome, _) = run_one(response("length", "写到一半", None));
        assert!(outcome.reply_text.contains("截断"), "{}", outcome.reply_text);
        assert!(!outcome.reply_text.contains("思考"));
    });
}

#[test]
fn sensitive_is_an_error_even_with_partial_text_and_the_text_is_not_the_reply() {
    with_sandbox(|| {
        let (outcome, session) = run_one(response("sensitive", "半截正文", None));
        assert!(matches!(outcome.terminal, Terminal::Error));
        assert!(outcome.reply_text.contains("审核"), "{}", outcome.reply_text);
        assert!(!outcome.reply_text.contains("半截正文"));
        let last = session.steps.last().expect("error step");
        assert_eq!(last.finish_reason.as_deref(), Some("sensitive"));
    });
}

#[test]
fn the_other_known_failures_each_get_their_own_message() {
    with_sandbox(|| {
        let (inference, _) = run_one(response("network_error", "", None));
        let (context, _) = run_one(response("model_context_window_exceeded", "", None));
        assert!(inference.reply_text.contains("推理"), "{}", inference.reply_text);
        assert!(context.reply_text.contains("上下文"), "{}", context.reply_text);
        assert_ne!(inference.reply_text, context.reply_text);
    });
}

#[test]
fn unknown_finish_reason_with_text_is_a_normal_reply() {
    with_sandbox(|| {
        let (outcome, session) = run_one(response("end_turn", "你好呀", None));
        assert!(matches!(outcome.terminal, Terminal::None));
        assert_eq!(outcome.reply_text, "你好呀");
        assert_eq!(session.steps.last().unwrap().finish_reason.as_deref(), Some("end_turn"));
    });
}

#[test]
fn unknown_finish_reason_without_output_shows_the_raw_value() {
    with_sandbox(|| {
        let (outcome, session) = run_one(response("vendor_special", "", Some("只想了")));
        assert!(matches!(outcome.terminal, Terminal::Error));
        assert!(outcome.reply_text.contains("vendor_special"), "{}", outcome.reply_text);
        let last = session.steps.last().expect("error step");
        assert_eq!(last.reasoning_content.as_deref(), Some("只想了"));
    });
}

#[test]
fn silent_stop_after_a_read_only_tool_says_the_tool_ran_without_writing() {
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
    let mock = spawn_scripted_llm(vec![assistant_tools(call, None), response("stop", "", None)]);
    r#loop::try_set_binding_json(&json!({ "key": "spark" })).expect("Set spark binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "看看目录", &cfg_for(&mock));

    assert!(matches!(outcome.terminal, Terminal::Error));
    assert!(!outcome.wrote);
    assert!(outcome.reply_text.contains("已执行工具"), "{}", outcome.reply_text);
    assert!(outcome.reply_text.contains("未写入"));
    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}
