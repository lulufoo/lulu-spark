//! LW-273: a timed-out request is sent again by the LLM layer, over a real connection.

use super::*;

/// The first `silent` connections get no reply; the next one is answered.
fn serve_after_silent_connections(silent: usize) -> MockLlm {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let join = thread::spawn(move || {
        let mut held = Vec::new();
        for _ in 0..silent {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buf = [0u8; 65536];
            let _ = stream.read(&mut buf);
            held.push(stream);
        }
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 65536];
        let _ = stream.read(&mut buf);
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
        let _ = stream.write_all(headers.as_bytes());
        write_http_chunk(&mut stream, sse_content_chunk("ok", Some("stop")).as_bytes());
        write_http_chunk(&mut stream, b"data: [DONE]\n\n");
        write_http_chunk_end(&mut stream);
    });
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

fn call(mock: &MockLlm, max_retries: usize) -> llm::LlmCallOutcome {
    let options = llm::CallOptions {
        timeout: Duration::from_millis(250),
        max_retries,
        ..llm::CallOptions::default()
    };
    llm::chat_completions_with_options(
        &[json!({"role":"user","content":"x"})],
        &[],
        &options,
        &llm_config_for(mock),
        None,
        None,
    )
}

#[test]
fn a_timeout_followed_by_an_answer_succeeds() {
    with_agent_sandbox(|_| {
        let mock = serve_after_silent_connections(1);
        let outcome = call(&mock, 2);
        assert_eq!(outcome.error, None, "{outcome:?}");
        let content = outcome.snapshot.and_then(|message| message.content);
        assert_eq!(content.as_deref(), Some("ok"));
    });
}

#[test]
fn two_retries_are_not_enough_for_three_silent_connections() {
    with_agent_sandbox(|_| {
        let mock = serve_after_silent_connections(3);
        let outcome = call(&mock, 2);
        assert_eq!(outcome.error, Some(LlmError::Timeout));
    });
}

#[test]
fn without_a_retry_budget_the_first_timeout_is_final() {
    with_agent_sandbox(|_| {
        let mock = serve_after_silent_connections(1);
        let outcome = call(&mock, 0);
        assert_eq!(outcome.error, Some(LlmError::Timeout));
    });
}
