//! LW-262: the last LLM call of a Turn is sent with `tool_choice: "none"` and a wrap-up prompt.

use super::support::*;
use crate::agent::r#loop::MAX_LLM_CALLS_PER_TURN;
use crate::agent::session::{EXTRA_APPEND_CONTENT, EXTRA_SKIPPED};

fn tool_response(round: usize) -> (u16, Value) {
    assistant_tools(
        json!([{
            "id": format!("selection_{round}"),
            "type": "function",
            "function": { "name": "list_todo_tasks", "arguments": "{}" }
        }]),
        None,
    )
}

/// Runs one Turn whose first `MAX - 1` replies call a tool and whose last reply is `last`.
fn run_turn_to_the_limit(last: (u16, Value)) -> (TurnOutcome, session::Session, MockLlm, TestSandbox) {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    let (mcp, mcp_port) = start_isolated_mcp(&sandbox);
    register_test_mcp("spark", mcp_port);

    let mut responses: Vec<(u16, Value)> = (0..MAX_LLM_CALLS_PER_TURN - 1).map(tool_response).collect();
    responses.push(last);
    let mock = spawn_scripted_llm(responses);
    r#loop::try_set_binding_json(&json!({ "key": "spark" })).expect("Set spark binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "read the todo list again and again", &cfg_for(&mock));

    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
    (outcome, session, mock, sandbox)
}

fn tool_steps(session: &session::Session) -> Vec<&Step> {
    session.steps.iter().filter(|step| step.role == "tool").collect()
}

#[test]
fn only_the_last_call_forbids_tools_and_carries_the_wrap_up_prompt() {
    let (_, _, mock, _sandbox) = run_turn_to_the_limit(assistant_text("Most of it is done; the rest is below."));
    let hits = mock.hits.lock().unwrap();
    assert_eq!(hits.len(), MAX_LLM_CALLS_PER_TURN);

    for hit in &hits[..MAX_LLM_CALLS_PER_TURN - 1] {
        assert_eq!(hit["tool_choice"], "auto");
        assert!(!hit["messages"].to_string().contains("MAXIMUM STEPS REACHED"), "no wrap-up before the last call");
    }
    let last = &hits[MAX_LLM_CALLS_PER_TURN - 1];
    assert_eq!(last["tool_choice"], "none");
    assert!(last.get("tools").is_some(), "tools stay in the request to keep the prefix cacheable");
    let last_message = last["messages"].as_array().unwrap().last().unwrap();
    assert_eq!(last_message["role"], "user");
    assert!(last_message["content"].as_str().unwrap().contains("MAXIMUM STEPS REACHED"));
}

#[test]
fn text_on_the_last_call_ends_the_turn_normally() {
    let (outcome, session, _, _sandbox) = run_turn_to_the_limit(assistant_text("Most of it is done; the rest is below."));
    assert_outcome(&outcome, "none", false);
    assert_eq!(outcome.reply_text, "Most of it is done; the rest is below.");
    assert_eq!(tool_steps(&session).len(), MAX_LLM_CALLS_PER_TURN - 1);
    assert_eq!(session.steps.last().unwrap().content.as_deref(), Some("Most of it is done; the rest is below."));
}

#[test]
fn tool_calls_on_the_last_call_are_kept_as_skipped_steps_and_not_run() {
    let (outcome, session, mock, _sandbox) = run_turn_to_the_limit(assistant_tools(
        json!([{
            "id": "late_call",
            "type": "function",
            "function": { "name": "list_todo_tasks", "arguments": "{}" }
        }]),
        Some("Stopping here; the remaining tasks are below."),
    ));
    assert_outcome(&outcome, "none", false);
    assert_eq!(outcome.reply_text, "Stopping here; the remaining tasks are below.");
    assert_eq!(mock.hits.lock().unwrap().len(), MAX_LLM_CALLS_PER_TURN, "no request after the last call");

    let tools = tool_steps(&session);
    assert_eq!(tools.len(), MAX_LLM_CALLS_PER_TURN, "24 run + 1 skipped");
    let skipped = tools.last().unwrap();
    assert_eq!(skipped.tool_call_id.as_deref(), Some("late_call"));
    assert_eq!(skipped.extra.get(EXTRA_SKIPPED), Some(&json!(true)));
    assert!(tools[..tools.len() - 1].iter().all(|step| step.extra.get(EXTRA_SKIPPED).is_none()));
}

#[test]
fn tool_calls_without_text_on_the_last_call_end_as_error_with_fixed_copy() {
    let (outcome, session, _, _sandbox) = run_turn_to_the_limit(tool_response(MAX_LLM_CALLS_PER_TURN));
    assert_outcome(&outcome, "error", false);
    assert!(outcome.reply_text.contains("call limit"));
    assert_eq!(tool_steps(&session).len(), MAX_LLM_CALLS_PER_TURN);
    let last = session.steps.last().unwrap();
    assert_eq!(last.role, "assistant");
    assert_eq!(last.content.as_deref(), Some(outcome.reply_text.as_str()));
}

#[test]
fn the_wrap_up_prompt_is_stored_with_the_step_it_follows_and_replays_after_reload() {
    let (_, session, _, _sandbox) = run_turn_to_the_limit(assistant_text("Summary."));
    let carrier = session
        .steps
        .iter()
        .position(|step| step.extra.contains_key(EXTRA_APPEND_CONTENT))
        .expect("a step carries the prompt");
    assert_eq!(session.steps[carrier].role, "tool");
    assert_eq!(carrier, session.steps.len() - 2, "it rides on the step just before the last reply");

    let reloaded = session::load_session(&session.session_id).expect("reload");
    assert_eq!(reloaded.steps, session.steps, "what was sent is what is stored");

    let messages = r#loop::build_llm_messages_from_steps(&reloaded.steps, "sys");
    let user_prompts: Vec<_> = messages
        .iter()
        .filter(|m| m["role"] == "user" && m["content"].as_str().is_some_and(|c| c.contains("MAXIMUM STEPS REACHED")))
        .collect();
    assert_eq!(user_prompts.len(), 1);
    let at = messages.iter().position(|m| m["role"] == "user" && m["content"].as_str().is_some_and(|c| c.contains("MAXIMUM STEPS REACHED"))).unwrap();
    assert_eq!(messages[at - 1]["role"], "tool");
    assert_eq!(messages[at + 1]["role"], "assistant");
}
