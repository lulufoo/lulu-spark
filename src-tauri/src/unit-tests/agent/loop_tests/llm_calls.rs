//! LW-172: the loop records every request of a Turn under the session's `llm-calls/`.

use super::support::*;

fn records(session_id: &str) -> Vec<Value> {
    let dir = session::session_llm_calls_dir(session_id).expect("dir");
    let mut names: Vec<_> = fs::read_dir(&dir)
        .expect("llm-calls dir exists")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    names.sort();
    names
        .iter()
        .map(|path| serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap())
        .collect()
}

#[test]
fn each_request_of_a_tool_turn_is_recorded_with_its_round() {
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
    let mut first = assistant_tools(call, None);
    first.1["choices"][0]["message"]["reasoning_content"] = json!("先看目录");
    let mock = spawn_scripted_llm(vec![first, assistant_text("目录是空的。")]);
    r#loop::try_set_binding_json(&json!({ "key": "spark" })).expect("Set spark binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "看看目录", &cfg_for(&mock));
    assert_eq!(outcome.reply_text, "目录是空的。");

    let recs = records(&session.session_id);
    assert_eq!(recs.len(), 2);
    assert_eq!(recs[0]["round"], 1);
    assert_eq!(recs[1]["round"], 2);
    assert_eq!(recs[0]["purpose"], "turn");
    assert_eq!(recs[0]["trace_id"], recs[1]["trace_id"]);
    assert!(recs[0]["trace_id"].is_string());
    assert!(recs[1]["step_index"].as_u64() > recs[0]["step_index"].as_u64());
    assert_eq!(recs[0]["response"]["message"]["reasoning_content"], "先看目录");
    assert_eq!(recs[1]["response"]["message"]["content"], "目录是空的。");
    // The second request carries the first response and the tool result.
    let sent = recs[1]["request"]["body"]["messages"].as_array().unwrap();
    assert!(sent.iter().any(|m| m["role"] == "tool"));
    assert!(sent.iter().any(|m| m["role"] == "assistant" && m["tool_calls"].is_array()));
    // Never inside the directory the model can write to.
    let scratch = sandbox.cache_dir().join("agent-scratch");
    assert!(!session::session_llm_calls_dir(&session.session_id).unwrap().starts_with(scratch));

    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn a_failed_request_in_a_turn_is_recorded_too() {
    with_sandbox(|| {
        let mock = spawn_scripted_llm(vec![(500, json!({ "error": { "message": "boom" } }))]);
        let master = create_bound_plan("主计划");
        let mut session = session::create_session().expect("session");
        arm_plan_binding(&master);
        let outcome = r#loop::run_loop(&mut session, "你好", &cfg_for(&mock));
        assert!(matches!(outcome.terminal, Terminal::Error));

        let recs = records(&session.session_id);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0]["outcome"]["category"], "status");
        assert_eq!(recs[0]["response"]["http_status"], 500);
    });
}
