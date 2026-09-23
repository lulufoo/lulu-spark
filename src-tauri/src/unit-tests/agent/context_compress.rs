use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::agent::context::count::count_tokens;
use crate::agent::context::render::render_request;
use crate::agent::context::window::{round_percent, window_tokens};
use crate::agent::llm::LlmConfig;
use crate::agent::session::{self, Turn};
use crate::agent::turn::{build_llm_messages, cancelled_turn_outcome};
use crate::test_support::TestSandbox;

use super::compress::{
    fit_summary_messages, maybe_compress, pack_summary_messages, should_compress,
    SUMMARY_SYSTEM_PROMPT,
};

const GLM_WINDOW: u64 = 1_048_576;
const TOKENS_AT_69: i64 = 728_760;
const TOKENS_AT_70: i64 = 728_761;

fn user_turn(text: &str) -> Turn {
    Turn {
        role: "user".into(),
        content: Some(text.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    }
}

fn assistant_turn(text: &str) -> Turn {
    Turn {
        role: "assistant".into(),
        content: Some(text.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    }
}

fn tool_turn(name: &str, content: &str) -> Turn {
    Turn {
        role: "tool".into(),
        content: Some(content.into()),
        tool_call_id: Some("call_1".into()),
        tool_calls: None,
        name: Some(name.into()),
    }
}

fn transcript_with_tool(result: &str) -> Vec<Turn> {
    vec![
        user_turn("hello"),
        Turn {
            role: "assistant".into(),
            content: None,
            tool_call_id: None,
            tool_calls: Some(json!([{
                "id": "call_1",
                "type": "function",
                "function": { "name": "lookup", "arguments": "{}" }
            }])),
            name: None,
        },
        tool_turn("lookup", result),
        assistant_turn("done"),
    ]
}

fn pack_tokens(turns: &[Turn]) -> u64 {
    let messages = pack_summary_messages(turns);
    count_tokens(&render_request(&messages, &[])).expect("count") as u64
}

#[test]
fn trigger_is_70_percent_of_the_glm_window() {
    assert_eq!(window_tokens("glm-5.2"), Some(GLM_WINDOW));
    assert_eq!(round_percent(TOKENS_AT_69 as u64, GLM_WINDOW), Some(69));
    assert_eq!(round_percent(TOKENS_AT_70 as u64, GLM_WINDOW), Some(70));
    assert!(!should_compress(TOKENS_AT_69, GLM_WINDOW));
    assert!(should_compress(TOKENS_AT_70, GLM_WINDOW));
    assert!(!should_compress(-1, GLM_WINDOW));
    assert!(!should_compress(TOKENS_AT_70, 0));
}

#[test]
fn summary_pack_keeps_user_assistant_and_tool_text() {
    let messages = pack_summary_messages(&transcript_with_tool("tool-body"));
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], SUMMARY_SYSTEM_PROMPT);
    assert_eq!(messages[1]["role"], "user");
    let pack = messages[1]["content"].as_str().expect("pack");
    assert!(pack.contains("hello"));
    assert!(pack.contains("lookup"));
    assert!(pack.contains("tool-body"));
    assert!(pack.contains("done"));
}

#[test]
fn fit_leaves_tool_result_when_already_under_the_window() {
    let turns = transcript_with_tool("short-result");
    let tokens = pack_tokens(&turns);
    let fitted = fit_summary_messages(&turns, tokens + 1).expect("fits");
    assert!(fitted[1]["content"].as_str().expect("pack").contains("short-result"));
}

#[test]
fn fit_truncates_the_longest_tool_result_when_at_the_window() {
    let long = "R".repeat(8_000);
    let turns = transcript_with_tool(&long);
    let tokens = pack_tokens(&turns);
    let fitted = fit_summary_messages(&turns, tokens).expect("fits after cut");
    let pack = fitted[1]["content"].as_str().expect("pack");
    assert!(pack.contains("hello"));
    assert!(pack.contains("done"));
    assert!(!pack.contains(&long));
    assert!(
        count_tokens(&render_request(&fitted, &[])).expect("count") as u64 + 1 <= tokens
    );
}

#[test]
fn fit_keeps_shrinking_user_text_when_tools_are_gone() {
    let turns = vec![user_turn("hello"), assistant_turn("done")];
    let tokens = pack_tokens(&turns);
    let fitted = fit_summary_messages(&turns, tokens).expect("still fits");
    assert!(count_tokens(&render_request(&fitted, &[])).expect("count") as u64 + 1 <= tokens);
    assert_eq!(fitted[0]["content"], SUMMARY_SYSTEM_PROMPT);
}

#[test]
fn fit_gives_up_only_when_the_empty_pack_still_fills_the_window() {
    let turns = vec![user_turn("hello"), assistant_turn("done")];
    assert!(fit_summary_messages(&turns, 1).is_none());
}

#[test]
fn replace_turns_keeps_messages_steps_and_last_prompt() {
    let _sandbox = TestSandbox::new();
    let session = session::create_session().expect("create");
    let mut loaded = session::load_session(&session.session_id).expect("load");
    loaded.turns = transcript_with_tool("raw-result");
    session::save_session(&loaded).expect("save");
    session::store_last_prompt(&session.session_id, 100, "[]").expect("snapshot");
    let ui_before = session::load_turns_value(&session.session_id);
    let path = session::session_file_path(&session.session_id).expect("path");
    let (steps_before, messages_before) = table_counts(&path);

    session::replace_turns_with_summary(&session.session_id, 1, 1, "compressed body")
        .expect("replace");

    assert_eq!(session::load_turns_value(&session.session_id), ui_before);
    assert_eq!(
        session::load_last_prompt_tokens(&session.session_id).expect("tokens"),
        Some(100)
    );
    let (steps_after, messages_after) = table_counts(&path);
    assert_eq!(steps_after, steps_before);
    assert_eq!(messages_after, messages_before);
    assert_eq!(
        session::load_summary_bodies(&session.session_id).expect("bodies"),
        vec!["compressed body".to_string()]
    );
    assert_eq!(
        session::active_turn_seq_range(&session.session_id).expect("range"),
        None
    );
    let reloaded = session::load_session(&session.session_id).expect("reload");
    assert!(reloaded.turns.is_empty());
    let conn = Connection::open(&path).expect("open");
    let turns: i64 = conn
        .query_row("SELECT COUNT(*) FROM model_turns", [], |row| row.get(0))
        .expect("turns");
    assert_eq!(turns, 0);
}

#[test]
fn main_request_puts_summary_bodies_before_remaining_turns() {
    let messages = build_llm_messages(
        &[user_turn("later")],
        "system-prompt",
        &["early summary".into()],
    );
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "system-prompt");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "early summary");
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], "later");
    assert!(!serde_json::to_string(&messages).unwrap().contains("hello"));
}

#[test]
fn next_turn_seq_continues_past_the_replaced_range() {
    let _sandbox = TestSandbox::new();
    let session = session::create_session().expect("create");
    session::append_turn(&session.session_id, user_turn("first")).expect("append");
    session::replace_turns_with_summary(&session.session_id, 1, 1, "body").expect("replace");
    session::append_turn(&session.session_id, user_turn("next")).expect("append next");
    let path = session::session_file_path(&session.session_id).expect("path");
    let conn = Connection::open(&path).expect("open");
    let seq: i64 = conn
        .query_row("SELECT seq FROM model_turns", [], |row| row.get(0))
        .expect("seq");
    assert_eq!(seq, 2);
}

#[test]
fn maybe_compress_at_70_writes_summary_and_leaves_the_snapshot() {
    let _sandbox = TestSandbox::new();
    let mut session = session::create_session().expect("create");
    session.turns = transcript_with_tool("raw-result");
    session::save_session(&session).expect("save");
    session::store_last_prompt(&session.session_id, TOKENS_AT_70, "[]").expect("snapshot");
    let mock = spawn_summary_llm(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": { "role": "assistant", "content": "one-line summary" }
        }]
    }));
    maybe_compress(&mut session, &summary_cfg(&mock));
    assert!(session.turns.is_empty());
    assert_eq!(
        session::load_summary_bodies(&session.session_id).expect("bodies"),
        vec!["one-line summary".to_string()]
    );
    assert_eq!(
        session::load_last_prompt_tokens(&session.session_id).expect("tokens"),
        Some(TOKENS_AT_70)
    );
    let hits = mock.hits.lock().unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].body.get("tools").is_none());
    let sent = hits[0].body["messages"].as_array().expect("messages");
    assert_eq!(sent[0]["content"], SUMMARY_SYSTEM_PROMPT);
}

#[test]
fn maybe_compress_at_69_does_not_call_the_model() {
    let _sandbox = TestSandbox::new();
    let mut session = session::create_session().expect("create");
    session.turns = vec![user_turn("hello"), assistant_turn("done")];
    session::save_session(&session).expect("save");
    session::store_last_prompt(&session.session_id, TOKENS_AT_69, "[]").expect("snapshot");
    maybe_compress(
        &mut session,
        &LlmConfig {
            api_key: "sk-test".into(),
            base_url: "http://127.0.0.1:1".into(),
            model: "glm-5.2".into(),
        },
    );
    assert_eq!(session.turns.len(), 2);
    assert!(session::load_summary_bodies(&session.session_id)
        .expect("bodies")
        .is_empty());
}

#[test]
fn cancel_after_compress_drops_the_new_user_at_the_refreshed_checkpoint() {
    let _sandbox = TestSandbox::new();
    let mut session = session::create_session().expect("create");
    session.turns = vec![user_turn("hello"), assistant_turn("done")];
    session::save_session(&session).expect("save");
    session::store_last_prompt(&session.session_id, TOKENS_AT_70, "[]").expect("snapshot");
    let mock = spawn_summary_llm(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": { "role": "assistant", "content": "one-line summary" }
        }]
    }));
    maybe_compress(&mut session, &summary_cfg(&mock));
    let turns_checkpoint = session.turns.len();
    session.turns.push(user_turn("later"));
    cancelled_turn_outcome(&mut session, turns_checkpoint);
    assert!(session.turns.is_empty());
    let reloaded = session::load_session(&session.session_id).expect("reload");
    assert!(reloaded.turns.is_empty());
    assert_eq!(
        session::load_summary_bodies(&session.session_id).expect("bodies"),
        vec!["one-line summary".to_string()]
    );
}

#[test]
fn maybe_compress_failure_leaves_turns() {
    let _sandbox = TestSandbox::new();
    let mut session = session::create_session().expect("create");
    session.turns = vec![user_turn("hello"), assistant_turn("done")];
    session::save_session(&session).expect("save");
    session::store_last_prompt(&session.session_id, TOKENS_AT_70, "[]").expect("snapshot");
    let mock = spawn_summary_llm(json!({"error": {"message": "no"}}));
    maybe_compress(&mut session, &summary_cfg(&mock));
    assert_eq!(session.turns.len(), 2);
    assert!(session::load_summary_bodies(&session.session_id)
        .expect("bodies")
        .is_empty());
}

fn table_counts(path: &std::path::Path) -> (i64, i64) {
    let conn = Connection::open(path).expect("open");
    let steps: i64 = conn
        .query_row("SELECT COUNT(*) FROM model_steps", [], |row| row.get(0))
        .expect("steps");
    let messages: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .expect("messages");
    (steps, messages)
}

struct MockLlm {
    port: u16,
    hits: Arc<Mutex<Vec<CapturedRequest>>>,
    _join: thread::JoinHandle<()>,
}

struct CapturedRequest {
    body: Value,
}

fn summary_cfg(mock: &MockLlm) -> LlmConfig {
    LlmConfig {
        api_key: "sk-test".into(),
        base_url: format!("http://127.0.0.1:{}", mock.port),
        model: "glm-5.2".into(),
    }
}

fn spawn_summary_llm(resp: Value) -> MockLlm {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_thread = hits.clone();
    let join = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 65536];
        let n = stream.read(&mut buf).unwrap_or(0);
        let raw = String::from_utf8_lossy(&buf[..n]);
        let body_str = raw.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or("");
        let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
            .unwrap_or(json!({}));
        hits_thread.lock().unwrap().push(CapturedRequest { body });
        let (status, content_type, body) = if resp.pointer("/choices/0/message").is_some() {
            let chunk = json!({
                "choices": [{
                    "index": 0,
                    "delta": resp["choices"][0]["message"].clone(),
                    "finish_reason": resp["choices"][0].get("finish_reason").cloned().unwrap_or(Value::Null)
                }]
            });
            (
                200u16,
                "text/event-stream",
                format!("data: {chunk}\n\ndata: [DONE]\n\n"),
            )
        } else {
            (500u16, "application/json", resp.to_string())
        };
        let reply = format!(
            "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes());
    });
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}
