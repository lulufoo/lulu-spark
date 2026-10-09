//! LW-170: the loop stores `reasoning_content` and the response fields, and echoes the
//! thinking only inside the current Turn (after the last user Step).

use super::support::*;

/// Add thinking and the response-level `model` / `usage` to a scripted response.
fn thinking(mut response: (u16, Value), text: &str) -> (u16, Value) {
    response.1["choices"][0]["message"]["reasoning_content"] = json!(text);
    response.1["model"] = json!("glm-reply");
    response.1["usage"] = json!({ "prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5 });
    response
}

fn step(role: &str, content: &str, thinking: Option<&str>) -> Step {
    Step {
        role: role.into(),
        content: Some(content.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: thinking.map(str::to_string),
    }
}

fn thinking_of(message: &Value) -> Option<&str> {
    message.get("reasoning_content").and_then(Value::as_str)
}

#[test]
fn history_echoes_thinking_only_after_the_last_user_step() {
    let mut calls = step("assistant", "", Some("第二轮先读"));
    calls.tool_calls = Some(json!([{
        "id": "call_1", "type": "function",
        "function": { "name": "read", "arguments": "{}" }
    }]));
    let steps = vec![
        step("user", "第一轮", None),
        step("assistant", "第一轮答复", Some("第一轮的想法")),
        step("user", "第二轮", None),
        calls,
        step("assistant", "第二轮答复", Some("第二轮收尾")),
    ];
    let messages = r#loop::build_llm_messages_from_steps(&steps, "sys");
    let echoed: Vec<Option<&str>> = messages.iter().map(thinking_of).collect();
    // system, user, assistant, user, tool_call, assistant
    assert_eq!(
        echoed,
        [None, None, None, None, Some("第二轮先读"), Some("第二轮收尾")]
    );
}

#[test]
fn history_without_thinking_is_unchanged() {
    let steps = vec![step("user", "hi", None), step("assistant", "hello", None)];
    let messages = r#loop::build_llm_messages_from_steps(&steps, "sys");
    assert!(messages.iter().all(|m| m.get("reasoning_content").is_none()));
}

#[test]
fn tool_loop_echoes_thinking_in_the_same_turn_and_stores_the_response_fields() {
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
    let mock = spawn_scripted_llm(vec![
        thinking(assistant_tools(call, None), "先看目录"),
        thinking(assistant_text("目录是空的。"), "已经看到了"),
    ]);
    r#loop::try_set_binding_json(&json!({ "key": "spark" })).expect("Set spark binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "看看目录", &cfg_for(&mock));
    assert_eq!(outcome.reply_text, "目录是空的。");

    let hits = mock.hits.lock().unwrap();
    assert_eq!(hits.len(), 2);
    assert!(
        !hits[0].to_string().contains("reasoning_content"),
        "the first request has no earlier thinking to send"
    );
    let echoed = hits[1]["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|m| m.get("tool_calls").is_some())
        .expect("assistant tool_calls message");
    assert_eq!(thinking_of(echoed), Some("先看目录"));
    drop(hits);

    let calls = session.steps.iter().find(|s| s.tool_calls.is_some()).expect("tool_call step");
    assert_eq!(calls.reasoning_content.as_deref(), Some("先看目录"));
    assert_eq!(calls.finish_reason.as_deref(), Some("tool_calls"));
    assert_eq!(calls.model.as_deref(), Some("glm-reply"));
    assert_eq!(calls.usage.as_ref().map(|u| u["total_tokens"].clone()), Some(json!(5)));
    let last = session.steps.last().expect("final step");
    assert_eq!(last.reasoning_content.as_deref(), Some("已经看到了"));
    assert_eq!(last.finish_reason.as_deref(), Some("stop"));
    assert!(session.steps.iter().all(|s| s.role != "reasoning"));

    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn next_turn_does_not_send_the_previous_turns_thinking() {
    with_sandbox(|| {
        let mock = spawn_scripted_llm(vec![
            thinking(assistant_text("第一轮答复"), "第一轮的想法"),
            assistant_text("第二轮答复"),
        ]);
        let master = create_bound_plan("主计划");
        let mut session = session::create_session().expect("session");
        arm_plan_binding(&master);
        r#loop::run_loop(&mut session, "第一轮", &cfg_for(&mock));
        r#loop::run_loop(&mut session, "第二轮", &cfg_for(&mock));

        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 2);
        assert!(!hits[1].to_string().contains("第一轮的想法"));
        assert!(!hits[1].to_string().contains("reasoning_content"));
        assert_eq!(
            session.steps[1].reasoning_content.as_deref(),
            Some("第一轮的想法"),
            "stored for review even though it is not sent again"
        );
    });
}

#[test]
fn empty_response_fallback_step_carries_thinking_and_response_fields() {
    with_sandbox(|| {
        let empty = (
            200,
            json!({ "choices": [{
                "finish_reason": "stop",
                "message": { "role": "assistant", "content": "" }
            }] }),
        );
        let mock = spawn_scripted_llm(vec![thinking(empty, "只想不说")]);
        let master = create_bound_plan("主计划");
        let mut session = session::create_session().expect("session");
        arm_plan_binding(&master);
        let outcome = r#loop::run_loop(&mut session, "你好", &cfg_for(&mock));
        assert_eq!(outcome.reply_text, "模型响应为空，未执行任何写入。");

        let last = session.steps.last().expect("fallback step");
        assert_eq!(last.content.as_deref(), Some("模型响应为空，未执行任何写入。"));
        assert_eq!(last.reasoning_content.as_deref(), Some("只想不说"));
        assert_eq!(last.finish_reason.as_deref(), Some("stop"));
        assert_eq!(last.model.as_deref(), Some("glm-reply"));
    });
}
