//! Agent module tests (Session / Tools / LLM / Loop).

#[path = "loop_src.rs"]
mod loop_src;

mod loop_tests;

#[path = "engine_router_tests.rs"]
mod engine_router_tests;

#[path = "runtime_tests.rs"]
mod runtime_tests;

#[path = "ai_assistant_session_tests.rs"]
mod ai_assistant_session_tests;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::agent::llm::{self, LlmConfig, LlmError};
use crate::agent::session::{self, Session, Turn};
use crate::agent::SPARK_HOST_SYSTEM_PROMPT;
use crate::test_support::{with_config_test_serial, TestSandbox};

fn with_agent_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    f(&sandbox);
}

/// Minimal OpenAI-shaped tool defs for LLM HTTP tests (not Host business SSOT).
fn sample_openai_tool_defs() -> Vec<Value> {
    vec![json!({
        "type": "function",
        "function": {
            "name": "sample_tool",
            "description": "unit-test fixture tool",
            "parameters": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }
    })]
}


fn assert_under_cache_not_knowledge_root(path: &Path, sandbox: &TestSandbox) {
    let cache = sandbox.cache_dir();
    let knowledge = sandbox.knowledge_root();
    assert!(
        path.starts_with(&cache),
        "path {path:?} must be under cache_dir {cache:?}"
    );
    assert!(
        !path.starts_with(&knowledge),
        "path {path:?} must not be under knowledge_root {knowledge:?}"
    );
}

// ── Module mount ─────────────────────────────────────────────────────────────

#[test]
fn agent_module_mount_point_is_addressable() {
    let _ = SPARK_HOST_SYSTEM_PROMPT;
    let _ = std::any::type_name::<Session>();
}

// ── Session ──────────────────────────────────────────────────────────────────

#[test]
fn assistant_conversation_dir_requires_sandbox_and_misses_prod_cache() {
    with_config_test_serial(|| {
        assert!(
            !settings::is_test_sandbox(),
            "this isolation test must run outside TestSandbox"
        );
        let err = session::sessions_dir().expect_err("unisolated sessions_dir");
        assert!(
            err.contains("TestSandbox isolation"),
            "unisolated error should name the sandbox gate, got {err}"
        );
        assert!(
            session::create_session().is_err(),
            "create_session must refuse the user's conversation directory"
        );
    });

    with_agent_sandbox(|sandbox| {
        let dir = session::sessions_dir().expect("isolated sessions_dir");
        let expected = sandbox.cache_dir().join("agent").join("sessions");
        assert_eq!(dir, expected);
        sandbox
            .assert_not_prod_path(&dir)
            .expect("sessions_dir must not be under prod roots");
        let sess = session::create_session().expect("create in sandbox");
        let path = session::session_file_path(&sess.session_id).expect("path");
        assert!(path.starts_with(&expected));
        sandbox
            .assert_not_prod_path(&path)
            .expect("session file must not be under prod roots");
    });
}

#[test]
fn session_save_load_roundtrip_under_cache_agent_sessions() {
    with_agent_sandbox(|sandbox| {
        let sess = session::create_session().expect("create");
        assert!(!sess.session_id.is_empty());
        assert_eq!(sess.turns.len(), 0);

        let path = session::session_file_path(&sess.session_id).expect("path");
        assert_under_cache_not_knowledge_root(&path, sandbox);
        let expected_dir = sandbox.cache_dir().join("agent").join("sessions");
        assert!(
            path.starts_with(&expected_dir),
            "expected under {expected_dir:?}, got {path:?}"
        );
        assert!(path.is_file());

        session::append_turn(
            &sess.session_id,
            Turn {
                role: "user".into(),
                content: Some("hello".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append");

        let loaded = session::load_session(&sess.session_id).expect("load");
        assert_eq!(loaded.turns.len(), 1);
        assert_eq!(loaded.turns[0].role, "user");
        assert_eq!(loaded.turns[0].content.as_deref(), Some("hello"));
    });
}

#[test]
fn create_session_id_is_spark_chat_plus_32_hex() {
    with_agent_sandbox(|_| {
        let sess = session::create_session().expect("create");
        let id = sess.session_id;
        let suffix = id
            .strip_prefix("spark_chat_")
            .expect("new session id must start with spark_chat_");
        assert_eq!(suffix.len(), 32);
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "suffix must be 32 lowercase hex, got {suffix:?}"
        );
    });
}

#[test]
fn load_session_still_reads_legacy_sess_id() {
    with_agent_sandbox(|_| {
        let legacy = Session {
            session_id: "sess_84dafc26cec6".into(),
            turns: Vec::new(),
            staged: Vec::new(),
            llm: None,
        };
        session::save_session(&legacy).expect("save legacy");
        let loaded = session::load_session("sess_84dafc26cec6").expect("load legacy");
        assert_eq!(loaded.session_id, "sess_84dafc26cec6");
    });
}

#[test]
fn agent_error_log_writes_under_cache_agent_without_secrets() {
    with_agent_sandbox(|sandbox| {
        session::log_agent_error(
            "upstream failed Authorization: Bearer sk-SECRET api_key=sk-SECRET body={\"api_key\":\"sk-SECRET\"}",
        )
        .expect("log");

        let log_dir = session::agent_log_dir().expect("log dir");
        assert_under_cache_not_knowledge_root(&log_dir, sandbox);
        assert!(
            log_dir.ends_with("agent") || log_dir.file_name().and_then(|n| n.to_str()) == Some("agent"),
            "log dir should be cache/agent, got {log_dir:?}"
        );

        let mut found = false;
        for entry in fs::read_dir(&log_dir).expect("read log dir") {
            let entry = entry.expect("entry");
            if entry.file_type().expect("ft").is_file() {
                let text = fs::read_to_string(entry.path()).expect("read");
                assert!(
                    !text.contains("Bearer sk-SECRET"),
                    "log must not contain Authorization bearer secret"
                );
                assert!(
                    !text.contains("api_key=sk-SECRET") && !text.contains("\"api_key\":\"sk-SECRET\""),
                    "log must not contain api_key secret"
                );
                found = true;
            }
        }
        assert!(found, "expected at least one log file under {log_dir:?}");
    });
}

#[test]
fn assistant_diagnostic_log_is_versioned_jsonl_with_safe_metadata_only() {
    with_agent_sandbox(|sandbox| {
        let trace_id = TraceId::parse("trace_12345678").expect("valid trace id");
        let event = DiagnosticEvent::timing(
            "assistant.runtime",
            "turn.completed",
            &trace_id,
            Duration::from_millis(42),
        )
        .with_session_id("sess_abcdef123456")
        .with_static_field("adapter", "host")
        .with_bool_field("persisted", true);

        diagnostics::log(event).expect("write diagnostic event");

        let path = diagnostics::diagnostic_log_path().expect("diagnostic log path");
        assert_under_cache_not_knowledge_root(&path, sandbox);
        let log = fs::read_to_string(&path).expect("read diagnostic log");
        let line = log
            .lines()
            .last()
            .expect("diagnostic line");
        let value: Value = serde_json::from_str(line).expect("valid JSONL event");

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["component"], "assistant.runtime");
        assert_eq!(value["event"], "turn.completed");
        assert_eq!(value["trace_id"], "trace_12345678");
        assert_eq!(value["session_id"], "sess_abcdef123456");
        assert_eq!(value["elapsed_ms"], 42);
        assert_eq!(value["fields"]["adapter"], "host");
        assert_eq!(value["fields"]["persisted"], true);
        assert!(
            value.get("message").is_none()
                && value.get("prompt").is_none()
                && value.get("content").is_none(),
            "diagnostic event must not expose message content"
        );

        let startup = DiagnosticEvent::point("assistant.startup", "agent_loop_warm.scheduled", &trace_id);
        let startup_line = serde_json::to_value(&startup).expect("serialize startup event");
        assert!(startup_line.get("session_id").is_none());
        assert!(
            DiagnosticEvent::point("assistant.runtime", "turn.started", &trace_id)
                .with_session_id("")
                .session_id
                .is_none()
        );
    });
}

#[test]
fn assistant_diagnostics_are_correlated_without_message_content() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root");
    let command = fs::read_to_string(repo_root.join("src-tauri/src/commands/ai_assistant.rs"))
        .expect("read assistant command");
    let runtime = fs::read_to_string(repo_root.join("src-tauri/src/agent/runtime.rs"))
        .expect("read assistant runtime");
    let diagnostics =
        fs::read_to_string(repo_root.join("src-tauri/src/agent/diagnostics.rs"))
            .expect("read diagnostics");

    assert!(
        command.contains("trace_id: Option<String>")
            && command.contains("agent_chat_turn_with_trace"),
        "Tauri command must carry a caller correlation id into the runtime"
    );
    assert!(
        !command.contains("record_ai_assistant_timing"),
        "retired independent-window UI timing must not remain on the command surface"
    );
    assert!(
        runtime.contains("dispatch_from_settings")
            && runtime.contains("assistant.host"),
        "Host execution must resolve through the single runtime route"
    );
    assert!(
        !diagnostics.contains("with_dynamic_field"),
        "diagnostics must not provide an API for arbitrary dynamic strings"
    );
    assert!(
        diagnostics.contains("with_bounded_text_field"),
        "diagnostics must allow bounded error classification text"
    );
}

#[test]
fn assistant_diagnostics_bounded_error_text_redacts_secrets_and_newlines() {
    let cleaned = diagnostics::sanitize_bounded_text(
        "quota exceeded\nbody sk-abc123 Authorization: Bearer sk-SECRET",
    );
    assert!(!cleaned.contains('\n'));
    assert!(cleaned.contains("quota exceeded"));
    assert!(!cleaned.contains("sk-SECRET"));
    assert!(!cleaned.contains("sk-abc123"));
    assert!(cleaned.contains("sk-[REDACTED]"));

    let trace_id = TraceId::parse("trace_12345678").expect("valid trace id");
    let event = DiagnosticEvent::point("assistant.host", "turn.completed", &trace_id)
        .with_bounded_text_field("prompt", "should-not-appear")
        .with_bounded_text_field("sdk_error_message", "rate_limit: quota exceeded");
    assert!(event.fields.get("prompt").is_none());
    assert_eq!(
        event.fields.get("sdk_error_message").and_then(|v| v.as_str()),
        Some("rate_limit: quota exceeded")
    );
}

// ── Tools (legacy tools.rs removed; tools/ + mcp/ are turn SSOT) ──────────────

#[test]
fn t3_agent_legacy_tools_rs_module_removed() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/agent/tools.rs");
    assert!(
        !path.exists(),
        "legacy agent/tools.rs OpenAI plan defs must be deleted"
    );
    let mod_src = include_str!("../../agent/mod.rs");
    assert!(
        mod_src.contains("pub mod tools"),
        "agent/mod.rs must declare tools/ module"
    );
    assert!(
        !mod_src.contains("pub mod fs_tools") && !mod_src.contains("pub mod mcp_client"),
        "agent/mod.rs must not keep fs_tools / mcp_client"
    );
}


// ── LLM ──────────────────────────────────────────────────────────────────────

struct MockLlm {
    port: u16,
    hits: Arc<Mutex<Vec<CapturedRequest>>>,
    _join: thread::JoinHandle<()>,
}

struct CapturedRequest {
    path: String,
    authorization: Option<String>,
    body: Value,
}

fn spawn_mock_llm<F>(handler: F) -> MockLlm
where
    F: Fn(&CapturedRequest) -> (u16, Value) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_thread = hits.clone();
    let join = thread::spawn(move || {
        // Handle a handful of requests then exit.
        for _ in 0..8 {
            listener.set_nonblocking(false).ok();
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = [0u8; 65536];
            let n = stream.read(&mut buf).unwrap_or(0);
            let raw = String::from_utf8_lossy(&buf[..n]);
            let (headers, body_str) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_ref(), ""));
            let first = headers.lines().next().unwrap_or("");
            let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
            let authorization = headers
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
                .map(|l| l.split_once(':').unwrap().1.trim().to_string());
            let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
                .unwrap_or(json!({}));
            let captured = CapturedRequest {
                path,
                authorization,
                body,
            };
            let (status, resp) = handler(&captured);
            hits_thread.lock().unwrap().push(captured);
            let resp = mock_llm_http_response(status, &resp);
            let _ = stream.write_all(resp.as_bytes());
        }
    });
    // Give the thread a moment to listen.
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

fn mock_llm_http_response(status: u16, resp: &Value) -> String {
    let (content_type, body) =
        if (200..300).contains(&status) && resp.pointer("/choices/0/message").is_some() {
            ("text/event-stream", completion_json_to_sse(resp))
        } else {
            ("application/json", resp.to_string())
        };
    format!(
        "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn completion_json_to_sse(resp: &Value) -> String {
    let choice = &resp["choices"][0];
    let message = &choice["message"];
    let mut delta = serde_json::Map::new();
    if let Some(role) = message.get("role") {
        delta.insert("role".into(), role.clone());
    }
    match message.get("content") {
        Some(Value::Null) | None => {}
        Some(content) => {
            delta.insert("content".into(), content.clone());
        }
    }
    if let Some(calls) = message.get("tool_calls") {
        delta.insert("tool_calls".into(), calls.clone());
    }
    let chunk = json!({
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": choice.get("finish_reason").cloned().unwrap_or(Value::Null)
        }]
    });
    format!("data: {chunk}\n\ndata: [DONE]\n\n")
}

fn llm_config_for(mock: &MockLlm) -> LlmConfig {
    LlmConfig {
        api_key: "sk-test-key".into(),
        base_url: format!("http://127.0.0.1:{}", mock.port),
        model: "test-model".into(),
    }
}

#[test]
fn llm_chat_completions_sends_openai_compatible_stream_request() {
    with_agent_sandbox(|_| {
        let mock = spawn_mock_llm(|_req| {
            (
                200,
                json!({
                    "choices": [{
                        "finish_reason": "stop",
                        "message": { "role": "assistant", "content": "hi" }
                    }]
                }),
            )
        });
        let cfg = llm_config_for(&mock);
        let tools_defs = sample_openai_tool_defs();
        let messages = vec![json!({"role":"user","content":"ping"})];
        let msg = llm::chat_completions(&messages, &tools_defs, &cfg).expect("chat");
        assert_eq!(msg.content.as_deref(), Some("hi"));

        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0].path.ends_with("/v1/chat/completions"),
            "path={}",
            hits[0].path
        );
        assert_eq!(
            hits[0].authorization.as_deref(),
            Some("Bearer sk-test-key")
        );
        assert_eq!(hits[0].body["stream"], true);
        assert_eq!(hits[0].body["tool_choice"], "auto");
        assert_eq!(hits[0].body["model"], "test-model");
        let sent_tools = hits[0].body["tools"].as_array().expect("tools");
        assert_eq!(sent_tools.len(), 1);
    });
}

#[test]
fn llm_missing_config_does_not_send_http() {
    with_agent_sandbox(|_| {
        let hits = Arc::new(Mutex::new(0u32));
        let hits2 = hits.clone();
        let mock = spawn_mock_llm(move |_req| {
            *hits2.lock().unwrap() += 1;
            (200, json!({"choices":[{"message":{"content":"x"}}]}))
        });

        for cfg in [
            LlmConfig {
                api_key: "".into(),
                base_url: format!("http://127.0.0.1:{}", mock.port),
                model: "m".into(),
            },
            LlmConfig {
                api_key: "k".into(),
                base_url: "".into(),
                model: "m".into(),
            },
            LlmConfig {
                api_key: "k".into(),
                base_url: format!("http://127.0.0.1:{}", mock.port),
                model: "".into(),
            },
        ] {
            let err = llm::chat_completions(&[json!({"role":"user","content":"x"})], &[], &cfg)
                .expect_err("must not call");
            assert!(
                matches!(err, LlmError::MissingConfig),
                "expected MissingConfig, got {err:?}"
            );
        }
        assert_eq!(*hits.lock().unwrap(), 0);
    });
}

#[test]
fn llm_error_taxonomy_no_retry_and_length_not_continued() {
    with_agent_sandbox(|_| {
        // length
        {
            let mock = spawn_mock_llm(|_req| {
                (
                    200,
                    json!({
                        "choices": [{
                            "finish_reason": "length",
                            "message": { "role": "assistant", "content": "cut" }
                        }]
                    }),
                )
            });
            let err = llm::chat_completions(
                &[json!({"role":"user","content":"x"})],
                &sample_openai_tool_defs(),
                &llm_config_for(&mock),
            )
            .expect_err("length");
            assert!(matches!(err, LlmError::Truncated), "{err:?}");
            assert_eq!(mock.hits.lock().unwrap().len(), 1, "no auto-continue");
        }

        for (status, expect) in [
            (401u16, "unauthorized"),
            (403, "unauthorized"),
            (400, "bad_request"),
            (429, "rate_limited"),
            (500, "server"),
            (503, "server"),
        ] {
            let mock = spawn_mock_llm(move |_req| (status, json!({"error":{"message":"x"}})));
            let err = llm::chat_completions(
                &[json!({"role":"user","content":"x"})],
                &[],
                &llm_config_for(&mock),
            )
            .expect_err("http err");
            match (expect, &err) {
                ("unauthorized", LlmError::Unauthorized) => {}
                ("bad_request", LlmError::BadRequest(_)) => {}
                ("rate_limited", LlmError::RateLimited) => {}
                ("server", LlmError::Server(_)) => {}
                _ => panic!("status {status}: unexpected {err:?}"),
            }
            assert_eq!(mock.hits.lock().unwrap().len(), 1, "no retry for {status}");
        }
    });
}

#[test]
fn llm_timeout_is_typed_and_not_retried() {
    with_agent_sandbox(|_| {
        // Bind but never accept → client should time out (DEFAULT_TIMEOUT idle cap; test overrides).
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        // Keep listener alive without accepting.
        let _keep = listener;
        let cfg = LlmConfig {
            api_key: "k".into(),
            base_url: format!("http://127.0.0.1:{port}"),
            model: "m".into(),
        };
        // Use short timeout override for unit test speed when available.
        let err = llm::chat_completions_with_timeout(
            &[json!({"role":"user","content":"x"})],
            &[],
            &cfg,
            Duration::from_millis(200),
            None,
        )
        .expect_err("timeout");
        assert!(matches!(err, LlmError::Timeout), "{err:?}");
    });
}

fn spawn_sse_writer<F>(write_body: F) -> MockLlm
where
    F: FnOnce(&mut std::net::TcpStream) + Send + 'static,
{
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
        let (headers, body_str) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_ref(), ""));
        let first = headers.lines().next().unwrap_or("");
        let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
        let authorization = headers
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
            .map(|l| l.split_once(':').unwrap().1.trim().to_string());
        let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
            .unwrap_or(json!({}));
        hits_thread.lock().unwrap().push(CapturedRequest {
            path,
            authorization,
            body,
        });
        write_body(&mut stream);
    });
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

fn sse_content_chunk(content: &str, finish: Option<&str>) -> String {
    let chunk = json!({
        "choices": [{
            "index": 0,
            "delta": { "content": content },
            "finish_reason": finish
        }]
    });
    format!("data: {chunk}\n\n")
}

fn write_http_chunk(stream: &mut std::net::TcpStream, data: &[u8]) {
    let _ = write!(stream, "{:x}\r\n", data.len());
    let _ = stream.write_all(data);
    let _ = stream.write_all(b"\r\n");
    let _ = stream.flush();
}

fn write_http_chunk_end(stream: &mut std::net::TcpStream) {
    let _ = stream.write_all(b"0\r\n\r\n");
    let _ = stream.flush();
}

#[test]
fn llm_assembles_sse_content_and_tool_call_deltas() {
    with_agent_sandbox(|_| {
        let mock = spawn_sse_writer(|stream| {
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
            write_http_chunk(stream, sse_content_chunk("Hel", None).as_bytes());
            write_http_chunk(stream, sse_content_chunk("lo", None).as_bytes());
            let tool_name = json!({
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "id": "call_1",
                            "type": "function",
                            "function": { "name": "read", "arguments": "" }
                        }]
                    },
                    "finish_reason": null
                }]
            });
            let tool_args = json!({
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "function": { "arguments": "{\"path\":\"/tmp/a\"}" }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }]
            });
            write_http_chunk(stream, format!("data: {tool_name}\n\n").as_bytes());
            write_http_chunk(stream, format!("data: {tool_args}\n\n").as_bytes());
            write_http_chunk(stream, b"data: [DONE]\n\n");
            write_http_chunk_end(stream);
        });
        let msg = llm::chat_completions(
            &[json!({"role":"user","content":"x"})],
            &sample_openai_tool_defs(),
            &llm_config_for(&mock),
        )
        .expect("assembled");
        assert_eq!(msg.content.as_deref(), Some("Hello"));
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].id, "call_1");
        assert_eq!(msg.tool_calls[0].name, "read");
        assert_eq!(msg.tool_calls[0].arguments, "{\"path\":\"/tmp/a\"}");
        assert_eq!(msg.finish_reason.as_deref(), Some("tool_calls"));
    });
}

#[test]
fn llm_on_delta_receives_assembled_hints() {
    with_agent_sandbox(|_| {
        let mock = spawn_sse_writer(|stream| {
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
            write_http_chunk(stream, sse_content_chunk("你好", None).as_bytes());
            let last = json!({
                "choices": [{
                    "index": 0,
                    "delta": { "role": "assistant", "content": "" },
                    "finish_reason": "stop"
                }]
            });
            write_http_chunk(stream, format!("data: {last}\n\n").as_bytes());
            write_http_chunk(stream, b"data: [DONE]\n\n");
            write_http_chunk_end(stream);
        });
        let hints = Arc::new(Mutex::new(Vec::new()));
        let slot = hints.clone();
        let mut on_delta = |hint: &str| slot.lock().unwrap().push(hint.to_string());
        let msg = llm::chat_completions_with_timeout(
            &[json!({"role":"user","content":"x"})],
            &[],
            &llm_config_for(&mock),
            Duration::from_secs(2),
            Some(&mut on_delta),
        )
        .expect("chat");
        assert_eq!(msg.content.as_deref(), Some("你好"));
        let hints = hints.lock().unwrap();
        assert!(
            hints.iter().any(|hint| hint == "Receiving… 你好"),
            "on_delta hints={hints:?}"
        );
    });
}

#[test]
fn llm_idle_timeout_after_first_sse_chunk() {
    with_agent_sandbox(|_| {
        let mock = spawn_sse_writer(|stream| {
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
            write_http_chunk(stream, sse_content_chunk("Hi", None).as_bytes());
            thread::sleep(Duration::from_secs(3));
        });
        let err = llm::chat_completions_with_timeout(
            &[json!({"role":"user","content":"x"})],
            &[],
            &llm_config_for(&mock),
            Duration::from_millis(250),
            None,
        )
        .expect_err("idle timeout");
        assert!(matches!(err, LlmError::Timeout), "{err:?}");
    });
}

#[test]
fn llm_keeps_reading_when_chunks_keep_arriving() {
    with_agent_sandbox(|_| {
        let mock = spawn_sse_writer(|stream| {
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
            write_http_chunk(stream, sse_content_chunk("A", None).as_bytes());
            thread::sleep(Duration::from_millis(180));
            write_http_chunk(stream, sse_content_chunk("B", None).as_bytes());
            thread::sleep(Duration::from_millis(180));
            let last = json!({
                "choices": [{
                    "index": 0,
                    "delta": { "content": "C" },
                    "finish_reason": "stop"
                }]
            });
            write_http_chunk(stream, format!("data: {last}\n\ndata: [DONE]\n\n").as_bytes());
            write_http_chunk_end(stream);
        });
        let started = std::time::Instant::now();
        let msg = llm::chat_completions_with_timeout(
            &[json!({"role":"user","content":"x"})],
            &[],
            &llm_config_for(&mock),
            Duration::from_millis(300),
            None,
        )
        .expect("kept reading");
        assert_eq!(msg.content.as_deref(), Some("ABC"));
        assert!(
            started.elapsed() > Duration::from_millis(300),
            "total elapsed must exceed the idle window"
        );
    });
}

#[test]
fn llm_unsupported_tool_calls_errors_without_prompt_json_fallback() {
    with_agent_sandbox(|_| {
        let mock = spawn_mock_llm(|_req| {
            (
                400,
                json!({
                    "error": {
                        "message": "tools / tool_calls is not supported",
                        "type": "invalid_request_error"
                    }
                }),
            )
        });
        let tools_defs = sample_openai_tool_defs();
        let err = llm::chat_completions(
            &[json!({"role":"user","content":"add a sub task"})],
            &tools_defs,
            &llm_config_for(&mock),
        )
        .expect_err("unsupported");
        assert!(matches!(err, LlmError::UnsupportedToolCalls), "{err:?}");

        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 1, "must not retry with prompt-JSON fallback");
        let messages = hits[0].body["messages"].as_array().cloned().unwrap_or_default();
        let blob = serde_json::to_string(&messages).unwrap();
        assert!(
            !blob.to_ascii_lowercase().contains("respond with json")
                && !blob.contains("```json")
                && !blob.contains("tool call as JSON"),
            "must not degrade by stuffing JSON instructions into prompt: {blob}"
        );
    });
}

#[test]
fn llm_load_config_reads_settings_and_secret() {
    with_agent_sandbox(|_| {
        let mut s = settings::load().expect("load");
        // Preset platform/base_url are not client-writable via apply_config_payload;
        // persist them on the settings struct to exercise Host load_llm_config.
        settings::upsert_llm_entry(
            &mut s.llm,
            "host",
            &settings::LlmSettings {
                platform: "glm".into(),
                base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
                model: "demo-model".into(),
            },
        )
        .expect("upsert host llm");
        settings::save(&s).expect("save llm settings");
        secrets::set_secret(KEY_LLM_API_KEY, "sk-from-secret").expect("set");

        let cfg = llm::load_llm_config().expect("cfg");
        assert_eq!(cfg.api_key, "sk-from-secret");
        assert_eq!(cfg.base_url, "https://open.bigmodel.cn/api/paas/v4");
        assert_eq!(cfg.model, "demo-model");
    });
}

#[test]
fn llm_load_config_rejects_non_glm_host_entry() {
    with_agent_sandbox(|_| {
        let mut s = settings::load().expect("load");
        settings::upsert_llm_entry(
            &mut s.llm,
            "host",
            &settings::LlmSettings {
                platform: "kimi".into(),
                base_url: "https://api.example.com".into(),
                model: "legacy-model".into(),
            },
        )
        .expect("upsert legacy host llm");
        settings::save(&s).expect("save llm settings");
        secrets::set_secret(KEY_LLM_API_KEY, "sk-from-secret").expect("set");

        let err = llm::load_llm_config().expect_err("non-GLM Host config");
        assert!(matches!(err, LlmError::MissingConfig), "{err:?}");
    });
}

// ── T4: load_llm_config reads current typed entry (model/base_url) ───────────

#[test]
fn llm_load_config_uses_active_host_entry_not_cursor_residue() {
    with_agent_sandbox(|_| {
        let mut s = settings::load().expect("load");
        s.assistant_engine = "host".into();
        settings::upsert_llm_entry(
            &mut s.llm,
            "host",
            &settings::LlmSettings {
                platform: "glm".into(),
                base_url: "https://host-only.example/v4".into(),
                model: "host-active-model".into(),
            },
        )
        .expect("upsert host");
        s.llm.push(settings::LlmSettingsEntry {
            engine_type: "cursor".into(),
            platform: "cursor_agent".into(),
            base_url: "https://cursor-residue.example".into(),
            model: "cursor-residue-model".into(),
        });
        settings::save(&s).expect("save");
        secrets::set_secret(KEY_LLM_API_KEY, "sk-host-slot").expect("set host key");

        let cfg = llm::load_llm_config().expect("cfg");
        assert_eq!(cfg.model, "host-active-model");
        assert_eq!(cfg.base_url, "https://host-only.example/v4");
        assert_ne!(cfg.model, "cursor-residue-model");
        assert_ne!(cfg.base_url, "https://cursor-residue.example");
        // API key stays on the Host/GLM slot.
        assert_eq!(cfg.api_key, "sk-host-slot");
    });
}

#[test]
fn llm_load_config_missing_or_empty_active_entry_is_missing_config_no_cross_type() {
    with_agent_sandbox(|_| {
        let mut s = settings::load().expect("load");
        s.assistant_engine = "host".into();
        // Cursor entry alone must not satisfy Host load.
        s.llm.push(settings::LlmSettingsEntry {
            engine_type: "cursor".into(),
            platform: String::new(),
            base_url: "https://cursor-only.example".into(),
            model: "cursor-only-model".into(),
        });
        settings::save(&s).expect("save");
        secrets::set_secret(KEY_LLM_API_KEY, "sk-present").expect("set");

        let err = llm::load_llm_config().expect_err("must not fall back to cursor entry");
        assert!(matches!(err, LlmError::MissingConfig), "{err:?}");
    });
}

#[test]
fn llm_load_config_blank_assistant_engine_is_missing_config() {
    with_agent_sandbox(|_| {
        let mut s = settings::load().expect("load");
        s.assistant_engine = "".into();
        settings::upsert_llm_entry(
            &mut s.llm,
            "host",
            &settings::LlmSettings {
                base_url: "https://blank-defaults-host.example/v4".into(),
                model: "blank-host-model".into(),
                ..Default::default()
            },
        )
        .expect("upsert host");
        s.llm.push(settings::LlmSettingsEntry {
            engine_type: "cursor".into(),
            platform: String::new(),
            base_url: "https://should-not-use.example".into(),
            model: "should-not-use".into(),
        });
        settings::save(&s).expect("save");
        secrets::set_secret(KEY_LLM_API_KEY, "sk-blank-host").expect("set");

        let err = llm::load_llm_config().expect_err("blank engine");
        assert!(matches!(err, LlmError::MissingConfig), "{err:?}");
    });
}
