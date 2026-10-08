use std::collections::BTreeSet;

use serde_json::{json, Value};

use super::count::{count_spans, count_tokens};
use super::render::{render_marked, render_request};
use super::window::{round_percent, window_tokens, DEFAULT_WINDOW_TOKENS};

const HELPFUL: &str = "[gMASK]<sop><|system|>Reasoning Effort: Max<|system|>You are helpful.<|assistant|><think>";

#[test]
fn glm_window_is_the_documented_size() {
    assert_eq!(window_tokens("glm-5.2"), 1_048_576);
    assert_eq!(window_tokens("other-model"), DEFAULT_WINDOW_TOKENS);
    assert_eq!(DEFAULT_WINDOW_TOKENS, 255_000);
}

#[test]
fn percent_rounds_half_up_and_keeps_values_over_100() {
    assert_eq!(round_percent(0, 1_048_576), Some(0));
    assert_eq!(round_percent(524_288, 1_048_576), Some(50));
    assert_eq!(round_percent(1, 200), Some(1));
    assert_eq!(round_percent(1, 8), Some(13));
    assert_eq!(round_percent(1, 3), Some(33));
    assert_eq!(round_percent(2, 3), Some(67));
    assert_eq!(round_percent(130, 100), Some(130));
    assert_eq!(round_percent(1, 0), None);
}

#[test]
fn render_matches_the_official_template_for_app_messages() {
    let helpful = json!([{ "role": "system", "content": "You are helpful." }]);
    assert_eq!(render_request(helpful.as_array().unwrap(), &[]), HELPFUL);

    let call = json!([
        { "role": "system", "content": "S" },
        { "role": "user", "content": "Hi" },
        {
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "id": "c1",
                "type": "function",
                "function": { "name": "lookup", "arguments": "{\"q\":\"x\",\"n\":2}" }
            }]
        },
        { "role": "tool", "content": "ok", "tool_call_id": "c1" }
    ]);
    assert_eq!(
        render_request(call.as_array().unwrap(), &[]),
        "[gMASK]<sop><|system|>Reasoning Effort: Max<|system|>S<|user|>Hi<|assistant|><think></think>\n<tool_call>lookup<arg_key>q</arg_key><arg_value>x</arg_value><arg_key>n</arg_key><arg_value>2</arg_value></tool_call>\n<|observation|><tool_response>ok</tool_response><|assistant|><think>"
    );

    let both = json!([
        { "role": "user", "content": "Hi" },
        {
            "role": "assistant",
            "content": "  Hello  ",
            "tool_calls": [
                { "function": { "name": "lookup", "arguments": "{\"q\":\"x\"}" } },
                { "function": { "name": "other", "arguments": "{\"k\":\"v\",\"n\":1}" } }
            ]
        }
    ]);
    assert_eq!(
        render_request(both.as_array().unwrap(), &[]),
        "[gMASK]<sop><|system|>Reasoning Effort: Max<|user|>Hi<|assistant|><think></think>Hello\n<tool_call>lookup<arg_key>q</arg_key><arg_value>x</arg_value></tool_call><tool_call>other<arg_key>k</arg_key><arg_value>v</arg_value><arg_key>n</arg_key><arg_value>1</arg_value></tool_call>\n<|assistant|><think>"
    );

    let tools = json!([{
        "type": "function",
        "function": {
            "name": "lookup",
            "description": "Find <id>",
            "parameters": {
                "type": "object",
                "properties": { "q": { "type": "string" } },
                "required": ["q"]
            }
        },
        "strict": true
    }]);
    let user = json!([{ "role": "user", "content": "Hi" }]);
    assert_eq!(
        render_request(user.as_array().unwrap(), tools.as_array().unwrap()),
        "[gMASK]<sop><|system|>Reasoning Effort: Max<|system|>\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n\n{\"description\": \"Find <id>\", \"name\": \"lookup\", \"parameters\": {\"properties\": {\"q\": {\"type\": \"string\"}}, \"required\": [\"q\"], \"type\": \"object\"}}\n\n\n</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}<arg_key>{arg-key-1}</arg_key><arg_value>{arg-value-1}</arg_value><arg_key>{arg-key-2}</arg_key><arg_value>{arg-value-2}</arg_value>...</tool_call><|user|>Hi<|assistant|><think>"
    );
}

#[test]
fn tokenizer_counts_special_tokens_once() {
    assert_eq!(count_tokens("[gMASK]").expect("count"), 1);
    assert_eq!(count_tokens("<sop>").expect("count"), 1);
    assert_eq!(count_tokens("<|user|>").expect("count"), 1);
    assert_eq!(count_tokens("<think>").expect("count"), 1);
    assert_eq!(count_tokens(HELPFUL).expect("count"), 16);
}

#[test]
fn unbound_session_omits_context_percent() {
    let _sandbox = crate::test_support::TestSandbox::new();
    crate::agent::session::reset_live_for_tests();
    let created = crate::agent::shell::create_chat_session_core().expect("create");
    assert!(created["session_id"].as_str().is_some());
    assert!(created.get("context_percent").is_none());
}

#[test]
fn sent_prompt_token_count_survives_a_later_save() {
    let _sandbox = crate::test_support::TestSandbox::new();
    let session = crate::agent::session::create_session().expect("create");
    let messages = json!([
        { "role": "system", "content": "You are helpful." },
        { "role": "user", "content": "Hi" }
    ]);
    let messages = messages.as_array().expect("messages");
    let expected = count_tokens(&render_request(messages, &[])).expect("count") as i64;
    super::record_sent_prompt(&session.session_id, messages, &[], &BTreeSet::new());
    assert_eq!(
        crate::agent::session::load_last_prompt_tokens(&session.session_id).expect("load"),
        Some(expected)
    );
    crate::agent::session::save_session(&session).expect("save");
    assert_eq!(
        crate::agent::session::load_last_prompt_tokens(&session.session_id).expect("reload"),
        Some(expected)
    );
}

#[test]
fn prompt_breakdown_splits_system_tools_mcp_and_conversation() {
    let messages = json!([
        { "role": "system", "content": "You are helpful." },
        { "role": "user", "content": "Hi" }
    ]);
    let tools = json!([
        {
            "type": "function",
            "function": { "name": "read_file", "description": "Read", "parameters": { "type": "object" } }
        },
        {
            "type": "function",
            "function": { "name": "list_notes", "description": "Notes", "parameters": { "type": "object" } }
        }
    ]);
    let messages = messages.as_array().expect("messages");
    let tools = tools.as_array().expect("tools");
    let mcp_names = BTreeSet::from(["list_notes".to_string()]);
    let (text, spans) = render_marked(messages, tools, &mcp_names);
    let categories = count_spans(&text, &spans).expect("spans");
    let total = count_tokens(&text).expect("total");
    let summed: usize = categories.iter().map(|(_, tokens)| *tokens).sum();
    assert_eq!(summed, total);
    assert!(tokens_of(&categories, "system_prompt") > 0);
    assert!(tokens_of(&categories, "tools") > 0);
    assert!(tokens_of(&categories, "mcp") > 0);
    assert!(tokens_of(&categories, "conversation") > 0);
    assert!(categories.iter().all(|(id, _)| {
        ["system_prompt", "tools", "mcp", "conversation", "other"].contains(id)
    }));
}

fn tokens_of(categories: &[(&str, usize)], id: &str) -> usize {
    categories
        .iter()
        .find(|(name, _)| *name == id)
        .map(|(_, tokens)| *tokens)
        .unwrap_or(0)
}

#[test]
fn stored_breakdown_survives_a_later_save() {
    let _sandbox = crate::test_support::TestSandbox::new();
    let session = crate::agent::session::create_session().expect("create");
    let messages = json!([
        { "role": "system", "content": "You are helpful." },
        { "role": "user", "content": "Hi" }
    ]);
    let tools = json!([{
        "type": "function",
        "function": { "name": "list_notes", "description": "Notes", "parameters": { "type": "object" } }
    }]);
    let mcp_names = BTreeSet::from(["list_notes".to_string()]);
    super::record_sent_prompt(
        &session.session_id,
        messages.as_array().expect("messages"),
        tools.as_array().expect("tools"),
        &mcp_names,
    );
    let raw = crate::agent::session::load_last_prompt_breakdown(&session.session_id)
        .expect("load")
        .expect("breakdown");
    let parsed: Value = serde_json::from_str(&raw).expect("json");
    let rows = parsed.as_array().expect("rows");
    assert!(rows.iter().any(|row| row["id"] == "mcp" && row["tokens"].as_u64().unwrap_or(0) > 0));
    assert!(rows.iter().all(|row| row["id"] != "rules" && row["id"] != "skills"));
    crate::agent::session::save_session(&session).expect("save");
    assert_eq!(
        crate::agent::session::load_last_prompt_breakdown(&session.session_id).expect("reload"),
        Some(raw)
    );
}
