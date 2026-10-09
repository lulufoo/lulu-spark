//! LW-172: every LLM request leaves one `llm-calls/NNNN.json` record, success or failure.

use super::*;

const SESSION: &str = "spark_chat_recordtest";

fn ctx(round: usize) -> llm::CallContext {
    llm::CallContext {
        session_id: SESSION.into(),
        trace_id: Some("trace-abcdef12".into()),
        purpose: "turn",
        round,
        step_index: 4,
    }
}

fn delta_chunk(delta: Value, finish: Value) -> Value {
    json!({ "model": "glm-x", "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }] })
}

fn serve_sse(chunks: Vec<Value>) -> MockLlm {
    spawn_sse_writer(move |stream| {
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
        let _ = stream.write_all(headers.as_bytes());
        for chunk in chunks {
            write_http_chunk(stream, format!("data: {chunk}\n\n").as_bytes());
            thread::sleep(Duration::from_millis(15));
        }
        write_http_chunk(stream, b"data: [DONE]\n\n");
        write_http_chunk_end(stream);
    })
}

fn call(mock: &MockLlm, messages: &[Value], ctx: Option<&llm::CallContext>) -> llm::LlmCallOutcome {
    llm::chat_completions_recorded(
        messages,
        &sample_openai_tool_defs(),
        &llm_config_for(mock),
        Duration::from_secs(2),
        None,
        ctx,
    )
}

fn user(text: &str) -> Vec<Value> {
    vec![json!({ "role": "user", "content": text })]
}

fn record(number: u32) -> Value {
    let path = session::session_llm_calls_dir(SESSION)
        .expect("dir")
        .join(format!("{number:04}.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("record is json")
}

#[test]
fn a_successful_call_records_request_response_and_timings() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![
            delta_chunk(json!({ "reasoning_content": "想一下" }), Value::Null),
            delta_chunk(json!({ "content": "好" }), json!("stop")),
            json!({ "model": "glm-x", "choices": [], "usage": { "total_tokens": 9 } }),
        ]);
        let outcome = call(&mock, &user("你好"), Some(&ctx(2)));
        assert!(outcome.error.is_none());

        let rec = record(1);
        assert_eq!(rec["session_id"], SESSION);
        assert_eq!(rec["trace_id"], "trace-abcdef12");
        assert_eq!(rec["purpose"], "turn");
        assert_eq!(rec["round"], 2);
        assert_eq!(rec["step_index"], 4);
        assert_eq!(rec["request"]["body"]["messages"][0]["content"], "你好");
        assert_eq!(rec["request"]["body"]["model"], "test-model");
        assert!(rec["request"]["body"]["tools"].is_array());
        assert_eq!(rec["response"]["http_status"], 200);
        let message = &rec["response"]["message"];
        assert_eq!(message["content"], "好");
        assert_eq!(message["reasoning_content"], "想一下");
        assert_eq!(message["finish_reason"], "stop");
        assert_eq!(message["model"], "glm-x");
        assert_eq!(message["usage"]["total_tokens"], 9);
        assert_eq!(rec["outcome"]["category"], "ok");
        let t = &rec["timing_ms"];
        let (reasoning, content, total) = (
            t["first_reasoning"].as_u64().expect("first reasoning"),
            t["first_content"].as_u64().expect("first content"),
            t["total"].as_u64().expect("total"),
        );
        assert!(t["first_byte"].as_u64().is_some());
        assert!(reasoning <= content && content <= total, "{t}");
    });
}

#[test]
fn records_are_numbered_per_session_and_the_api_key_is_never_stored() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "a" }), json!("stop"))]);
        call(&mock, &user("one"), Some(&ctx(1)));
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "b" }), json!("stop"))]);
        call(&mock, &user("two"), Some(&ctx(2)));

        assert_eq!(record(1)["request"]["body"]["messages"][0]["content"], "one");
        assert_eq!(record(2)["request"]["body"]["messages"][0]["content"], "two");
        let dir = session::session_llm_calls_dir(SESSION).unwrap();
        for name in ["0001.json", "0002.json"] {
            let text = fs::read_to_string(dir.join(name)).unwrap();
            assert!(!text.contains("sk-test-key"), "{name} leaked the key");
        }
    });
}

#[test]
fn an_http_status_failure_is_recorded_with_its_body() {
    with_agent_sandbox(|_| {
        let mock = spawn_mock_llm(|_| (500, json!({ "error": { "message": "upstream boom" } })));
        let outcome = call(&mock, &user("hi"), Some(&ctx(1)));
        assert_eq!(outcome.error, Some(LlmError::Server(500)));

        let rec = record(1);
        assert_eq!(rec["response"]["http_status"], 500);
        assert!(rec["response"]["message"].is_null());
        assert!(rec["response"]["error_body"].as_str().unwrap().contains("upstream boom"));
        assert_eq!(rec["outcome"]["category"], "status");
        assert!(rec["outcome"]["error"].as_str().unwrap().contains("Server"));
    });
}

#[test]
fn a_rejected_response_still_records_what_arrived() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "写到一半" }), json!("length"))]);
        let outcome = call(&mock, &user("hi"), Some(&ctx(1)));
        assert_eq!(outcome.error, Some(LlmError::Truncated));

        let rec = record(1);
        assert_eq!(rec["response"]["message"]["content"], "写到一半");
        assert_eq!(rec["response"]["message"]["finish_reason"], "length");
        assert_eq!(rec["outcome"]["category"], "truncated");
    });
}

#[test]
fn a_reply_with_nothing_in_it_is_recorded_as_empty() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({}), json!("stop"))]);
        call(&mock, &user("hi"), Some(&ctx(1)));
        assert_eq!(record(1)["outcome"]["category"], "empty");
    });
}

#[test]
fn secrets_inside_message_text_are_redacted_and_do_not_hang() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "ok" }), json!("stop"))]);
        let messages = user("Authorization: Bearer sk-live-123\nthen Bearer sk-two and Bearer sk-three, api_key=sk-four");
        call(&mock, &messages, Some(&ctx(1)));

        let dir = session::session_llm_calls_dir(SESSION).unwrap();
        let text = fs::read_to_string(dir.join("0001.json")).unwrap();
        for secret in ["sk-live-123", "sk-two", "sk-three", "sk-four"] {
            assert!(!text.contains(secret), "{secret} leaked");
        }
        assert!(text.contains("[REDACTED]"));
    });
}

#[test]
fn without_a_context_nothing_is_written() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "a" }), json!("stop"))]);
        let outcome = call(&mock, &user("hi"), None);
        assert!(outcome.error.is_none());
        assert!(!session::session_llm_calls_dir(SESSION).unwrap().exists());
    });
}

#[test]
fn a_record_that_cannot_be_written_does_not_change_the_result() {
    with_agent_sandbox(|_| {
        let dir = session::session_llm_calls_dir(SESSION).unwrap();
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        fs::write(&dir, b"a file where the directory should be").unwrap();

        let mock = serve_sse(vec![delta_chunk(json!({ "content": "a" }), json!("stop"))]);
        let outcome = call(&mock, &user("hi"), Some(&ctx(1)));
        assert!(outcome.error.is_none());
        assert_eq!(outcome.snapshot.unwrap().content.as_deref(), Some("a"));
    });
}

#[test]
fn an_invalid_session_id_does_not_change_the_result() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "a" }), json!("stop"))]);
        let mut bad = ctx(1);
        bad.session_id = "../escape".into();
        let outcome = call(&mock, &user("hi"), Some(&bad));
        assert!(outcome.error.is_none());
    });
}
