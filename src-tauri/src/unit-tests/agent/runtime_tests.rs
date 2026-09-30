//! Host-only chat runtime coverage.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::agent::engine_router;
use crate::agent::r#loop::{self, ChatTurnResult, EVENT_TURN_COMPLETED};
use crate::agent::runtime;
use crate::agent::session::value_exposes_engine_selection;
use crate::agent::WORKBENCH_HOST_SYSTEM_PROMPT;
use crate::mcp_host::registry as mcp_registry;
use crate::test_support::TestSandbox;

struct MockLlm {
    port: u16,
    hits: Arc<Mutex<Vec<Value>>>,
    _join: thread::JoinHandle<()>,
}

fn spawn_scripted_llm(body: Value) -> MockLlm {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_for_thread = hits.clone();
    let join = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("request");
        let mut buffer = [0_u8; 65536];
        let size = stream.read(&mut buffer).expect("read");
        let raw = String::from_utf8_lossy(&buffer[..size]);
        if let Some(index) = raw.find("\r\n\r\n") {
            if let Ok(request) = serde_json::from_str::<Value>(&raw[index + 4..]) {
                hits_for_thread.lock().expect("hits lock").push(request);
            }
        }
        let response = mock_llm_http_response(200, &body);
        stream.write_all(response.as_bytes()).expect("write");
    });
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

fn assistant_text(content: &str) -> Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": { "role": "assistant", "content": content }
        }]
    })
}

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    f();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
}

fn create_bound_plan(title: &str) -> String {
    format!("bind-{title}")
}

fn arm_plan_binding(master_task_id: &str) {
    r#loop::set_binding(r#loop::Binding {
        tools: json!([
            { "name": "get_plan", "ctx": { "master_task_id": master_task_id } },
            { "name": "list_sub_tasks", "ctx": { "master_task_id": master_task_id } },
            { "name": "add_sub_task", "ctx": { "master_task_id": master_task_id } },
            { "name": "update_sub_title", "ctx": { "master_task_id": master_task_id } },
            { "name": "update_master_title", "ctx": { "master_task_id": master_task_id } }
        ]),
        prompt: json!(WORKBENCH_HOST_SYSTEM_PROMPT),
        callbacks: json!({}),
    })
    .expect("binding");
}

fn install_host_llm(mock: &MockLlm) {
    let mut app_settings = settings::load().expect("load");
    settings::apply_config_payload(
        &mut app_settings,
        &json!({
            "assistant_engine": "host",
            "llm": { "model": "test-model" }
        }),
    )
    .expect("apply");
    settings::upsert_llm_entry(
        &mut app_settings.llm,
        "host",
        &settings::LlmSettings {
            platform: "glm".into(),
            base_url: format!("http://127.0.0.1:{}", mock.port),
            model: "test-model".into(),
        },
    )
    .expect("host entry");
    settings::save(&app_settings).expect("save");
    secrets::set_secret(KEY_LLM_API_KEY, "sk-host").expect("host key");
}

#[test]
fn chat_turn_signature_stays_engine_opaque_and_runtime_is_host_only() {
    let _: fn(&str, &str, Option<&str>) -> Result<ChatTurnResult, String> = runtime::chat_turn;
    let source = include_str!("../../agent/runtime.rs");
    assert!(!source.contains("cursor"));
    assert!(!source.contains("Cursor"));
    assert!(source.contains("dispatch_from_settings"));
    assert!(source.contains("run_loop"));
}

#[test]
fn configured_glm_chat_routes_through_agent_loop_without_cursor_runtime() {
    with_sandbox(|| {
        let master = create_bound_plan("runtime-host");
        let mock = spawn_scripted_llm(assistant_text("host reply"));
        install_host_llm(&mock);
        arm_plan_binding(&master);
        let opened = r#loop::ensure_chat_session_core().expect("open");
        let session_id = opened["session_id"].as_str().expect("session");

        let result = runtime::chat_turn(session_id, "hello", Some(&master)).expect("chat");

        assert_eq!(result.body["terminal"], "none");
        assert!(result.body["reply_text"]
            .as_str()
            .unwrap_or_default()
            .contains("host reply"));
        assert_eq!(mock.hits.lock().expect("hits").len(), 1);
        assert_eq!(result.emit_turn_completed.expect("event")["event"], EVENT_TURN_COMPLETED);
        assert!(!value_exposes_engine_selection(&result.body));
    });
}

#[test]
fn legacy_cursor_and_empty_settings_are_unconfigured_without_host_call() {
    with_sandbox(|| {
        let master = create_bound_plan("runtime-unconfigured");
        arm_plan_binding(&master);

        let mut app_settings = settings::load().expect("load");
        app_settings.assistant_engine = "cursor".into();
        settings::save(&app_settings).expect("save cursor legacy");
        let opened = r#loop::ensure_chat_session_core().expect("open");
        let session_id = opened["session_id"].as_str().expect("session");

        let result = runtime::chat_turn(session_id, "hello", Some(&master)).expect("chat");
        assert_eq!(result.body["terminal"], "error");
        assert!(result.body["reply_text"]
            .as_str()
            .unwrap_or_default()
            .contains("GLM is not configured"));

        let mut app_settings = settings::load().expect("reload");
        app_settings.assistant_engine.clear();
        settings::save(&app_settings).expect("save empty");
        assert!(matches!(
            engine_router::resolve_engine(&app_settings),
            Err(engine_router::EngineRouteError::Unconfigured)
        ));
    });
}

#[test]
fn busy_and_stale_session_gates_remain_owned_by_runtime() {
    with_sandbox(|| {
        let master = create_bound_plan("runtime-gates");
        arm_plan_binding(&master);
        let opened = r#loop::ensure_chat_session_core().expect("open");
        let session_id = opened["session_id"].as_str().expect("session").to_string();

        r#loop::set_busy_for_tests(true);
        let busy = runtime::chat_turn(&session_id, "second", Some(&master)).expect("busy");
        assert_eq!(busy.body["busy"], true);
        assert!(busy.emit_turn_completed.is_none());

        r#loop::set_busy_for_tests(false);
        let stale = runtime::chat_turn("not-live-session", "stale", Some(&master)).expect("stale");
        assert_eq!(stale.body["code"], "rejected_not_live_session");
        assert!(stale.emit_turn_completed.is_none());
    });
}
