//! Agent module tests (Session / Tools / LLM / Loop).

#[path = "loop_tests.rs"]
mod loop_tests;

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
use crate::services::agent::llm::{self, LlmConfig, LlmError};
use crate::services::agent::session::{self, Session, Turn};
use crate::services::agent::tools;
use crate::services::agent::PLAN_ASSISTANT_SYSTEM_PROMPT;
use crate::services::plan_task;
use crate::test_support::TestSandbox;

fn with_agent_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    f(&sandbox);
}

fn assert_tool_shell_ok(v: &Value) {
    assert_eq!(v["ok"], true, "expected ok shell, got {v}");
    assert!(v.get("data").is_some(), "missing data: {v}");
    assert!(v.get("error").is_none(), "unexpected error: {v}");
    assert!(v.get("code").is_none(), "unexpected code: {v}");
}

fn assert_tool_shell_err(v: &Value, code: &str) {
    assert_eq!(v["ok"], false, "expected err shell, got {v}");
    assert!(v.get("error").and_then(|e| e.as_str()).is_some(), "missing error: {v}");
    assert_eq!(v["code"], code, "code mismatch: {v}");
    assert!(v.get("data").is_none(), "unexpected data: {v}");
}

fn create_bound_plan(title: &str) -> String {
    let created = plan_task::create_master_with_subs(title, Some(&["子项A"]));
    assert_eq!(created["_status"], 201);
    created["master_task_id"].as_str().unwrap().to_string()
}

fn assert_under_cache_not_corpus(path: &Path, sandbox: &TestSandbox) {
    let cache = sandbox.cache_dir();
    let corpus = sandbox.knowledge_corpus_root();
    assert!(
        path.starts_with(&cache),
        "path {path:?} must be under cache_dir {cache:?}"
    );
    assert!(
        !path.starts_with(&corpus),
        "path {path:?} must not be under knowledge_corpus_root {corpus:?}"
    );
}

// ── Module mount ─────────────────────────────────────────────────────────────

#[test]
fn agent_module_mount_point_is_addressable() {
    let _ = PLAN_ASSISTANT_SYSTEM_PROMPT;
    let _ = std::any::type_name::<Session>();
    let defs = tools::openai_tool_definitions();
    assert_eq!(defs.len(), 5);
}

// ── Session ──────────────────────────────────────────────────────────────────

#[test]
fn session_save_load_roundtrip_under_cache_agent_sessions() {
    with_agent_sandbox(|sandbox| {
        let sess = session::create_session(Some("task_abc"), Some("标题")).expect("create");
        assert!(!sess.session_id.is_empty());
        assert_eq!(sess.bound_master_task_id.as_deref(), Some("task_abc"));
        assert_eq!(sess.turns.len(), 0);

        let path = session::session_file_path(&sess.session_id).expect("path");
        assert_under_cache_not_corpus(&path, sandbox);
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
fn agent_error_log_writes_under_cache_agent_without_secrets() {
    with_agent_sandbox(|sandbox| {
        session::log_agent_error(
            "upstream failed Authorization: Bearer sk-SECRET api_key=sk-SECRET body={\"api_key\":\"sk-SECRET\"}",
        )
        .expect("log");

        let log_dir = session::agent_log_dir().expect("log dir");
        assert_under_cache_not_corpus(&log_dir, sandbox);
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

// ── Tools ────────────────────────────────────────────────────────────────────

#[test]
fn tools_definitions_are_exactly_five_whitelist_names() {
    let defs = tools::openai_tool_definitions();
    let names: Vec<&str> = defs
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        names,
        vec![
            "get_plan",
            "list_sub_tasks",
            "add_sub_task",
            "update_sub_title",
            "update_master_title",
        ]
    );
}

#[test]
fn tools_five_suite_happy_path_and_data_omits_plan_md() {
    with_agent_sandbox(|_| {
        let master_id = create_bound_plan("主计划");

        let get = tools::dispatch("get_plan", &json!({}), Some(&master_id));
        assert_tool_shell_ok(&get);
        let data = &get["data"];
        assert_eq!(data["master_task_id"], master_id);
        assert_eq!(data["title"], "主计划");
        assert!(data.get("plan_md").is_none(), "success data must omit plan_md");
        assert!(data["sub_tasks"].as_array().unwrap().len() >= 1);

        let listed = tools::dispatch("list_sub_tasks", &json!({}), Some(&master_id));
        assert_tool_shell_ok(&listed);
        assert!(listed["data"].get("plan_md").is_none());
        let sub_id = listed["data"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap()
            .to_string();

        let added = tools::dispatch("add_sub_task", &json!({ "title": "新子项" }), Some(&master_id));
        assert_tool_shell_ok(&added);
        assert_eq!(added["data"]["title"], "新子项");
        assert!(added["data"].get("plan_md").is_none());

        let upd_sub = tools::dispatch(
            "update_sub_title",
            &json!({ "sub_task_id": sub_id, "title": "改后子标题" }),
            Some(&master_id),
        );
        assert_tool_shell_ok(&upd_sub);
        assert_eq!(upd_sub["data"]["title"], "改后子标题");

        let upd_master = tools::dispatch(
            "update_master_title",
            &json!({ "title": "改后主标题" }),
            Some(&master_id),
        );
        assert_tool_shell_ok(&upd_master);
        assert_eq!(upd_master["data"]["title"], "改后主标题");
        assert_eq!(upd_master["data"]["master_task_id"], master_id);
    });
}

#[test]
fn tools_forbidden_without_or_with_invalid_binding() {
    with_agent_sandbox(|_| {
        let _ = create_bound_plan("有计划但不绑定");
        for name in [
            "get_plan",
            "list_sub_tasks",
            "add_sub_task",
            "update_sub_title",
            "update_master_title",
        ] {
            let none = tools::dispatch(name, &json!({ "title": "x", "sub_task_id": "s" }), None);
            assert_tool_shell_err(&none, "forbidden");

            let empty = tools::dispatch(
                name,
                &json!({ "title": "x", "sub_task_id": "s" }),
                Some(""),
            );
            assert_tool_shell_err(&empty, "forbidden");

            let missing = tools::dispatch(
                name,
                &json!({ "title": "x", "sub_task_id": "s" }),
                Some("task_does_not_exist_zzzz"),
            );
            assert_tool_shell_err(&missing, "forbidden");
        }
    });
}

#[test]
fn tools_unknown_name_never_executes_returns_unsupported() {
    with_agent_sandbox(|_| {
        let master_id = create_bound_plan("绑定");
        let before = plan_task::get_by_id(&master_id);
        let before_title = before["title"].clone();

        let v = tools::dispatch(
            "delete_master",
            &json!({ "title": "should-not-run" }),
            Some(&master_id),
        );
        assert_tool_shell_err(&v, "unsupported");

        let after = plan_task::get_by_id(&master_id);
        assert_eq!(after["title"], before_title);
    });
}

#[test]
fn tools_error_codes_are_from_allowed_set() {
    with_agent_sandbox(|_| {
        let master_id = create_bound_plan("码表");
        let samples = vec![
            tools::dispatch("get_plan", &json!({}), None),
            tools::dispatch("add_sub_task", &json!({}), Some(&master_id)),
            tools::dispatch(
                "update_sub_title",
                &json!({ "sub_task_id": "nope", "title": "t" }),
                Some(&master_id),
            ),
            tools::dispatch("no_such_tool", &json!({}), Some(&master_id)),
        ];
        let allowed = ["not_found", "bad_request", "forbidden", "unsupported", "internal"];
        for v in samples {
            if v["ok"] == false {
                let code = v["code"].as_str().unwrap_or("");
                assert!(allowed.contains(&code), "unexpected code {code} in {v}");
            }
        }
    });
}

#[test]
fn tools_update_master_title_success_and_validation_failures() {
    with_agent_sandbox(|_| {
        let master_id = create_bound_plan("主标题校验");

        let ok_v = tools::dispatch(
            "update_master_title",
            &json!({ "title": "  合法标题  " }),
            Some(&master_id),
        );
        assert_tool_shell_ok(&ok_v);
        assert_eq!(ok_v["data"]["title"], "合法标题");
        assert_eq!(ok_v["data"]["master_task_id"], master_id);

        let blank = tools::dispatch(
            "update_master_title",
            &json!({ "title": "   \t  " }),
            Some(&master_id),
        );
        assert_tool_shell_err(&blank, "bad_request");

        // title_unit_count: English tokens (words), not characters — 21 words > TITLE_UNIT_LIMIT=20
        let over = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone";
        let too_long = tools::dispatch(
            "update_master_title",
            &json!({ "title": over }),
            Some(&master_id),
        );
        assert_tool_shell_err(&too_long, "bad_request");

        let after = plan_task::get_by_id(&master_id);
        assert_eq!(after["title"], "合法标题");
    });
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
            let body = resp.to_string();
            let resp = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
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

fn llm_config_for(mock: &MockLlm) -> LlmConfig {
    LlmConfig {
        api_key: "sk-test-key".into(),
        base_url: format!("http://127.0.0.1:{}", mock.port),
        model: "test-model".into(),
    }
}

#[test]
fn llm_chat_completions_sends_openai_compatible_non_stream_request() {
    with_agent_sandbox(|_| {
        let mock = spawn_mock_llm(|req| {
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
        let tools_defs = tools::openai_tool_definitions();
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
        assert_eq!(hits[0].body["stream"], false);
        assert_eq!(hits[0].body["tool_choice"], "auto");
        assert_eq!(hits[0].body["model"], "test-model");
        let sent_tools = hits[0].body["tools"].as_array().expect("tools");
        assert_eq!(sent_tools.len(), 5);
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
                &tools::openai_tool_definitions(),
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
        // Bind but never accept → client should time out (60s hard cap; test overrides via cfg helper if available).
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
        )
        .expect_err("timeout");
        assert!(matches!(err, LlmError::Timeout), "{err:?}");
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
        let tools_defs = tools::openai_tool_definitions();
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
        settings::apply_config_payload(
            &mut s,
            &json!({
                "llm": {
                    "base_url": "https://api.example.com",
                    "model": "demo-model",
                    "platform": "kimi"
                }
            }),
        );
        settings::save(&s).expect("save llm settings");
        secrets::set_secret(KEY_LLM_API_KEY, "sk-from-secret").expect("set");

        let cfg = llm::load_llm_config().expect("cfg");
        assert_eq!(cfg.api_key, "sk-from-secret");
        assert_eq!(cfg.base_url, "https://api.example.com");
        assert_eq!(cfg.model, "demo-model");
    });
}
