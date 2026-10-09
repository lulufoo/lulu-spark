//! LW-168: reasoning_content / model / usage / finish_reason survive SSE assembly.

use super::*;

fn serve_sse(chunks: Vec<Value>) -> MockLlm {
    spawn_sse_writer(move |stream| {
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
        let _ = stream.write_all(headers.as_bytes());
        for chunk in chunks {
            write_http_chunk(stream, format!("data: {chunk}\n\n").as_bytes());
        }
        write_http_chunk(stream, b"data: [DONE]\n\n");
        write_http_chunk_end(stream);
    })
}

fn delta_chunk(delta: Value, finish: Value) -> Value {
    json!({ "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }] })
}

fn call_traced(mock: &MockLlm, hints: Option<&mut dyn FnMut(&str, &str)>) -> llm::LlmCallOutcome {
    llm::chat_completions_traced(
        &[json!({"role":"user","content":"x"})],
        &sample_openai_tool_defs(),
        &llm_config_for(mock),
        Duration::from_secs(2),
        hints,
    )
}

#[test]
fn llm_carries_reasoning_model_and_usage_when_present() {
    with_agent_sandbox(|_| {
        let mut first = delta_chunk(json!({ "reasoning_content": "想" }), Value::Null);
        first["model"] = json!("glm-x");
        first["usage"] = Value::Null;
        let mock = serve_sse(vec![
            first,
            delta_chunk(json!({ "reasoning_content": "一下" }), Value::Null),
            delta_chunk(json!({ "content": "好" }), json!("stop")),
            json!({ "model": "glm-x", "choices": [],
                    "usage": { "prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5 } }),
        ]);
        let hints = Arc::new(Mutex::new(Vec::new()));
        let slot = hints.clone();
        let mut on_delta = |hint: &str, reasoning: &str| {
            slot.lock().unwrap().push((hint.to_string(), reasoning.to_string()));
        };
        let msg = call_traced(&mock, Some(&mut on_delta))
            .into_result()
            .expect("ok");
        assert_eq!(msg.reasoning_content.as_deref(), Some("想一下"));
        assert_eq!(msg.content.as_deref(), Some("好"));
        assert_eq!(msg.model.as_deref(), Some("glm-x"));
        assert_eq!(msg.finish_reason.as_deref(), Some("stop"));
        assert_eq!(msg.usage.as_ref().unwrap()["total_tokens"], 5);
        let hints = hints.lock().unwrap();
        assert_eq!(
            hints.first().map(|(hint, reasoning)| (hint.as_str(), reasoning.as_str())),
            Some(("Thinking…", "想"))
        );
        assert!(hints.iter().any(|(hint, reasoning)| {
            hint == "Thinking…" && reasoning == "想一下"
        }));
    });
}

#[test]
fn llm_leaves_new_fields_empty_when_upstream_sends_none() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "hi" }), json!("stop"))]);
        let msg = call_traced(&mock, None).into_result().expect("ok");
        assert_eq!(msg.content.as_deref(), Some("hi"));
        assert!(msg.reasoning_content.is_none());
        assert!(msg.model.is_none());
        assert!(msg.usage.is_none());
    });
}

#[test]
fn llm_reasoning_only_response_is_ok_and_keeps_reasoning() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![
            delta_chunk(json!({ "reasoning_content": "只想不说" }), Value::Null),
            delta_chunk(json!({}), json!("stop")),
        ]);
        let msg = call_traced(&mock, None).into_result().expect("ok");
        assert!(msg.content.is_none());
        assert!(msg.tool_calls.is_empty());
        assert_eq!(msg.reasoning_content.as_deref(), Some("只想不说"));
    });
}

#[test]
fn llm_length_error_still_exposes_full_snapshot() {
    with_agent_sandbox(|_| {
        let mut chunk = delta_chunk(json!({ "content": "cut" }), json!("length"));
        chunk["model"] = json!("m-1");
        chunk["usage"] = json!({ "total_tokens": 9 });
        let mock = serve_sse(vec![
            delta_chunk(json!({ "reasoning_content": "r" }), Value::Null),
            chunk,
        ]);
        let outcome = call_traced(&mock, None);
        assert_eq!(outcome.error, Some(LlmError::Truncated));
        let snap = outcome.snapshot.clone().expect("snapshot kept");
        assert_eq!(snap.content.as_deref(), Some("cut"));
        assert_eq!(snap.reasoning_content.as_deref(), Some("r"));
        assert_eq!(snap.model.as_deref(), Some("m-1"));
        assert_eq!(snap.usage.as_ref().unwrap()["total_tokens"], 9);
        assert_eq!(snap.finish_reason.as_deref(), Some("length"));
        // Legacy view is unchanged: still a typed error, no message.
        assert_eq!(outcome.into_result().unwrap_err(), LlmError::Truncated);
    });
}

#[test]
fn llm_malformed_tool_call_keeps_snapshot_and_reports_error() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(
            json!({ "tool_calls": [{ "index": 0, "function": { "arguments": "{\"a\":" } }] }),
            json!("tool_calls"),
        )]);
        let outcome = call_traced(&mock, None);
        assert!(matches!(outcome.error, Some(LlmError::InvalidResponse(_))));
        let snap = outcome.snapshot.expect("snapshot kept");
        assert_eq!(snap.tool_calls.len(), 1);
        assert_eq!(snap.tool_calls[0].arguments, "{\"a\":");
    });
}

#[test]
fn llm_keeps_unknown_finish_reason_verbatim() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "x" }), json!("vendor_special"))]);
        let msg = call_traced(&mock, None).into_result().expect("ok");
        assert_eq!(msg.finish_reason.as_deref(), Some("vendor_special"));
    });
}

#[test]
fn llm_http_status_error_has_no_snapshot() {
    with_agent_sandbox(|_| {
        let mock = spawn_mock_llm(|_req| (500, json!({"error":{"message":"boom"}})));
        let outcome = call_traced(&mock, None);
        assert!(outcome.snapshot.is_none());
        assert_eq!(outcome.error, Some(LlmError::Server(500)));
    });
}

#[test]
fn llm_request_body_is_unchanged_by_response_parsing() {
    with_agent_sandbox(|_| {
        let mock = serve_sse(vec![delta_chunk(json!({ "content": "x" }), json!("stop"))]);
        let _ = call_traced(&mock, None);
        let hits = mock.hits.lock().unwrap();
        let body = hits[0].body.as_object().expect("object body");
        assert!(!body.contains_key("stream_options"));
        assert!(!body.contains_key("max_tokens"));
    });
}
