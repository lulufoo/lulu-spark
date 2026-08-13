//! T4: engine-aware chat runtime — gate/busy/persist + Host∥Cursor routing.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings;
use crate::services::agent::cursor_adapter::{
    self, CursorErrorCode, CursorSessionRuntime, FakeCursorRunnerClient, TurnRequest,
    HOST_GENERIC_UPSTREAM_UNAVAILABLE,
};
use crate::services::agent::engine_router::{self, AdapterKind};
use crate::services::agent::r#loop::{self, ChatTurnResult, EVENT_TURN_COMPLETED};
use crate::services::agent::runtime;
use crate::services::agent::session::value_exposes_engine_selection;
use crate::services::agent::PLAN_ASSISTANT_SYSTEM_PROMPT;
use crate::services::mcp_endpoint_readiness::ReadyMcpTransports;
use crate::services::mcp_server_registry::{self, HttpMcpTransport, SEEDED_BUSINESS_KEY};
use crate::services::todo_task;
use crate::test_support::TestSandbox;

struct MockLlm {
    port: u16,
    hits: Arc<Mutex<Vec<Value>>>,
    _join: thread::JoinHandle<()>,
}

fn spawn_scripted_llm(responses: Vec<(u16, Value)>) -> MockLlm {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_bg = hits.clone();
    let join = thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 65536];
            let n = stream.read(&mut buf).unwrap_or(0);
            let raw = String::from_utf8_lossy(&buf[..n]);
            if let Some(idx) = raw.find("\r\n\r\n") {
                if let Ok(v) = serde_json::from_str::<Value>(&raw[idx + 4..]) {
                    hits_bg.lock().unwrap().push(v);
                }
            }
            let body_s = body.to_string();
            let resp = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_s}",
                body_s.len()
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

fn assistant_text(content: &str) -> (u16, Value) {
    (
        200,
        json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "role": "assistant", "content": content }
            }]
        }),
    )
}

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    runtime::reset_for_tests();
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();
    f();
    runtime::reset_for_tests();
    r#loop::reset_runtime_for_tests();
    mcp_server_registry::clear_for_tests();
}

fn create_bound_plan(title: &str) -> String {
    let created = todo_task::create_master_with_subs(title, Some(&["子项A"]));
    assert_eq!(created["_status"], 201);
    created["master_task_id"].as_str().unwrap().to_string()
}

fn plan_tools_binding(master: &str) -> r#loop::Binding {
    let tools = json!([
        { "name": "get_plan", "ctx": { "master_task_id": master } },
        { "name": "list_sub_tasks", "ctx": { "master_task_id": master } },
        { "name": "add_sub_task", "ctx": { "master_task_id": master } },
        { "name": "update_sub_title", "ctx": { "master_task_id": master } },
        { "name": "update_master_title", "ctx": { "master_task_id": master } },
    ]);
    r#loop::Binding {
        tools,
        prompt: json!(PLAN_ASSISTANT_SYSTEM_PROMPT),
        callbacks: json!({}),
    }
}

fn arm_plan_binding(master: &str) {
    r#loop::set_binding(plan_tools_binding(master)).expect("Set plan Binding");
}

fn install_host_llm(mock: &MockLlm) {
    let mut s = settings::load().expect("load");
    settings::apply_config_payload(
        &mut s,
        &json!({
            "assistant_engine": "host",
            "llm": { "model": "test-model" }
        }),
    )
    .expect("apply");
    let model = settings::llm_entry_by_type(&s.llm, "host")
        .map(|e| e.model.clone())
        .unwrap_or_else(|| "test-model".into());
    settings::upsert_llm_entry(
        &mut s.llm,
        "host",
        &settings::LlmSettings {
            platform: "openai_compatible".into(),
            base_url: format!("http://127.0.0.1:{}", mock.port),
            model,
        },
    )
    .expect("upsert host llm");
    settings::save(&s).expect("save");
    secrets::set_secret(KEY_LLM_API_KEY, "sk-test-host").expect("host key");
}

fn install_cursor_engine() {
    let mut s = settings::load().expect("load");
    settings::apply_config_payload(
        &mut s,
        &json!({
            "assistant_engine": "cursor",
            "llm": { "model": "composer-2.5" }
        }),
    )
    .expect("apply");
    settings::save(&s).expect("save");
    secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-test-cursor").expect("cursor key");
}

fn ready_mcp_sample() -> ReadyMcpTransports {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert(
        "Accept".into(),
        "application/json, text/event-stream".into(),
    );
    ReadyMcpTransports {
        transports: vec![HttpMcpTransport {
            name: "workbench".into(),
            url: "http://127.0.0.1:9876/mcp".into(),
            headers,
        }],
    }
}

fn install_fake_cursor_runtime() -> Arc<Mutex<Vec<cursor_adapter::FakeLogEntry>>> {
    let fake = FakeCursorRunnerClient::new();
    let log = fake.log.clone();
    let runtime_rt = CursorSessionRuntime::with_client_factory(move |_key| {
        let c = FakeCursorRunnerClient::from_shared(log.clone());
        Ok(Box::new(c) as Box<dyn cursor_adapter::CursorRunnerClient>)
    });
    runtime::set_cursor_runtime_for_tests(Some(Arc::new(runtime_rt)));
    runtime::set_profile_for_tests(Some(super::test_business_profile("composer-2.5")));
    fake.log.clone()
}

#[test]
fn t4_chat_turn_signature_is_engine_opaque() {
    let _: fn(&str, &str, Option<&str>) -> Result<ChatTurnResult, String> = runtime::chat_turn;
    let s = settings::AppSettings::default();
    let _ = engine_router::dispatch_from_settings(
        &s,
        &engine_router::TurnInput {
            session_id: "x".into(),
            message: "y".into(),
        },
        || Ok("h".into()),
        || Ok("c".into()),
    );
}

#[test]
fn t4_loop_llm_remain_host_only_zero_cursor_imports() {
    let loop_src = include_str!("../../../services/agent/loop.rs");
    let llm_src = include_str!("../../../services/agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        assert!(
            !src.contains("cursor_adapter")
                && !src.contains("CursorSessionRuntime")
                && !src.contains("cursor-agent-runner"),
            "{label} must stay Host-only (zero Cursor adapter/SDK imports)"
        );
    }
}

#[test]
fn t4_ai_assistant_routes_formal_chat_through_runtime() {
    let cmd = include_str!("../../../commands/ai_assistant.rs");
    assert!(
        cmd.contains("runtime::chat_turn") || cmd.contains("agent::runtime::chat_turn"),
        "ai_assistant formal chat must call runtime::chat_turn"
    );
    let agent_chat_fn = cmd
        .split("pub fn agent_chat_turn_json")
        .nth(1)
        .and_then(|s| s.split("#[tauri::command]").next())
        .unwrap_or("");
    assert!(
        agent_chat_fn.contains("runtime::chat_turn")
            || agent_chat_fn.contains("crate::services::agent::runtime::chat_turn"),
        "agent_chat_turn_json body must call runtime, got: {agent_chat_fn}"
    );
    assert!(
        !agent_chat_fn.contains("agent_chat_turn_core"),
        "agent_chat_turn_json must not call agent_chat_turn_core directly (production bug path)"
    );
}

#[test]
fn t4_host_path_keeps_tools_empty_and_skips_cursor_child() {
    with_sandbox(|| {
        let master = create_bound_plan("runtime-host");
        let mock = spawn_scripted_llm(vec![assistant_text("host-via-runtime")]);
        install_host_llm(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();

        let result = runtime::chat_turn(sid, "你好", Some(&master)).expect("host chat");
        assert_eq!(result.body["terminal"], "none");
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap_or("")
                .contains("host-via-runtime"),
            "reply={}",
            result.body["reply_text"]
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
        let req = &mock.hits.lock().unwrap()[0];
        match req.get("tools") {
            None => {}
            Some(Value::Array(a)) => assert!(a.is_empty(), "Host tools must be []"),
            Some(other) => panic!("unexpected tools: {other}"),
        }
        assert!(
            !runtime::cursor_runtime_installed_for_tests(),
            "Host path must not require Cursor runtime / Node child"
        );
        let ev = result.emit_turn_completed.expect("emit");
        assert_eq!(ev["event"], EVENT_TURN_COMPLETED);
        assert!(!value_exposes_engine_selection(&result.body));
    });
}

#[test]
fn t4_cursor_settings_must_not_call_host_llm_load_path() {
    with_sandbox(|| {
        let master = create_bound_plan("runtime-cursor");
        let mock = spawn_scripted_llm(vec![assistant_text("HOST-MUST-NOT-RUN")]);
        install_host_llm(&mock);
        install_cursor_engine();
        let log = install_fake_cursor_runtime();

        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        let before_mcp = r#loop::session_capability_mcp_config().expect("L2 face");
        let result = runtime::chat_turn(&sid, "cursor please", Some(&master)).expect("cursor");

        assert_eq!(
            mock.hits.lock().unwrap().len(),
            0,
            "Cursor settings must not hit Host HTTP / load_llm_config path"
        );
        assert!(
            result.body["reply_text"].as_str().unwrap_or("") == "fake-ok"
                || result
                    .body["reply_text"]
                    .as_str()
                    .unwrap_or("")
                    .contains("fake-ok"),
            "cursor reply expected, got {}",
            result.body["reply_text"]
        );
        assert_ne!(
            result.body["reply_text"].as_str(),
            Some("HOST-MUST-NOT-RUN")
        );
        let entries = log.lock().unwrap().clone();
        for method in ["create", "turn"] {
            let hit = entries
                .iter()
                .find(|e| e.method == method)
                .unwrap_or_else(|| panic!("Cursor adapter must run {method}, got {entries:?}"));
            assert!(
                hit.request_id.is_some(),
                "runtime CSR hook {method} must go through request (request_id), got {hit:?}"
            );
        }
        let after_mcp = r#loop::session_capability_mcp_config().expect("still");
        assert_eq!(after_mcp, before_mcp, "same-session MCP read face");
        assert!(!value_exposes_engine_selection(&result.body));
    });
}

#[test]
fn t4_cursor_failure_does_not_fallback_host_or_tools_dispatch() {
    with_sandbox(|| {
        let master = create_bound_plan("cursor-fail");
        let title_before = todo_task::get_by_id(&master)["title"].clone();
        let mock = spawn_scripted_llm(vec![assistant_text("HOST-FALLBACK-FORBIDDEN")]);
        install_host_llm(&mock);
        install_cursor_engine();

        let runtime_rt = CursorSessionRuntime::with_client_factory(move |_key| {
            Err(cursor_adapter::CursorError::new(
                CursorErrorCode::Runner,
                cursor_adapter::frontend_message_for(CursorErrorCode::Runner),
            ))
        });
        runtime::set_cursor_runtime_for_tests(Some(Arc::new(runtime_rt)));
        runtime::set_profile_for_tests(Some(super::test_business_profile("composer-2.5")));

        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();

        let result = runtime::chat_turn(sid, "fail please", Some(&master)).expect("surface Ok");
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(mock.hits.lock().unwrap().len(), 0, "no Host fallback");
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
        let reply = result.body["reply_text"].as_str().unwrap_or("");
        assert_ne!(reply, HOST_GENERIC_UPSTREAM_UNAVAILABLE);
        assert_eq!(
            reply,
            cursor_adapter::frontend_message_for(CursorErrorCode::Runner)
        );
        assert_ne!(result.body.get("adapter"), Some(&json!("host")));
    });
}

#[test]
fn t4_cursor_error_keeps_typed_mapping_not_host_generic() {
    with_sandbox(|| {
        let master = create_bound_plan("typed-err");
        install_cursor_engine();
        secrets::test_secrets_clear();
        let mut s = settings::load().expect("load");
        s.assistant_engine = "cursor".into();
        settings::save(&s).expect("save");

        runtime::set_cursor_runtime_for_tests(Some(Arc::new(
            CursorSessionRuntime::with_client_factory(move |_| {
                panic!("must not spawn when credential missing")
            }),
        )));
        runtime::set_profile_for_tests(Some(super::test_business_profile("composer-2.5")));

        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = runtime::chat_turn(sid, "no key", None).expect("ok envelope");
        assert_eq!(result.body["terminal"], "error");
        let reply = result.body["reply_text"].as_str().unwrap_or("");
        assert_eq!(
            reply,
            cursor_adapter::frontend_message_for(CursorErrorCode::Credential)
        );
        assert_ne!(reply, HOST_GENERIC_UPSTREAM_UNAVAILABLE);
        assert_ne!(reply, "上游服务暂时不可用，请稍后重试。");
    });
}

#[test]
fn t4_busy_and_session_gate_owned_by_runtime() {
    with_sandbox(|| {
        let master = create_bound_plan("gate");
        install_host_llm(&spawn_scripted_llm(vec![assistant_text("unused")]));
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        r#loop::set_busy_for_tests(true);
        let busy = runtime::chat_turn(&sid, "第二路", Some(&master)).unwrap();
        assert_eq!(busy.body["busy"], true);
        assert!(busy.emit_turn_completed.is_none());

        r#loop::set_busy_for_tests(false);
        let stale = runtime::chat_turn("not-live-session", "x", Some(&master)).unwrap();
        assert_eq!(stale.body["code"], "rejected_not_live_session");
        assert!(stale.emit_turn_completed.is_none());
    });
}

#[test]
fn t4_dispatch_from_settings_wires_host_and_cursor_closures() {
    let host_hits = std::sync::atomic::AtomicUsize::new(0);
    let cursor_hits = std::sync::atomic::AtomicUsize::new(0);
    let mut host_settings = settings::AppSettings::default();
    host_settings.assistant_engine = "host".into();
    let input = engine_router::TurnInput {
        session_id: "s".into(),
        message: "m".into(),
    };
    let host = engine_router::dispatch_from_settings(
        &host_settings,
        &input,
        || {
            host_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("H".into())
        },
        || {
            cursor_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("C".into())
        },
    )
    .expect("host");
    assert_eq!(host.adapter, AdapterKind::Host);
    assert_eq!(host_hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(cursor_hits.load(std::sync::atomic::Ordering::SeqCst), 0);

    let mut cursor_settings = settings::AppSettings::default();
    cursor_settings.assistant_engine = "cursor".into();
    let cursor = engine_router::dispatch_from_settings(
        &cursor_settings,
        &input,
        || {
            host_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("H".into())
        },
        || {
            cursor_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("C".into())
        },
    )
    .expect("cursor");
    assert_eq!(cursor.adapter, AdapterKind::Cursor);
    assert_eq!(cursor_hits.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn t4_runtime_source_uses_engine_router_and_not_tools_dispatch_on_cursor_fail() {
    let src = include_str!("../../../services/agent/runtime.rs");
    assert!(
        src.contains("dispatch_from_settings") || src.contains("route_chat_turn"),
        "runtime must wire via engine_router"
    );
    assert!(
        src.contains("run_loop"),
        "Host closure must call loop::run_loop"
    );
    assert!(
        !src.contains("tools::dispatch") && !src.contains("crate::services::agent::tools::"),
        "runtime must not call tools::dispatch on Cursor failure"
    );
    assert!(
        src.contains("load_llm_config"),
        "Host path may load_llm_config inside Host closure only"
    );
}

// ── T4 / AlignTests: runtime CSR hook stays request-only ─────────────────────

#[test]
fn t4_align_runtime_csr_hook_is_request_only() {
    let src = include_str!("../../../services/agent/runtime.rs");
    assert!(
        src.contains("set_cursor_runtime_for_tests") && src.contains("CursorSessionRuntime"),
        "CSR test hook type must remain (REWRITE, not delete)"
    );
    assert!(
        src.contains("request-only CSR"),
        "runtime test hook must mark request-only CSR alignment (typed create/turn/cancel gone)"
    );
    assert!(
        src.contains("rt.run_turn(req)") || src.contains(".run_turn(req)"),
        "test hook must still delegate to CSR.run_turn"
    );
    for deleted in [
        "CursorAgentSdk",
        "map_client_request_error",
        "engine_settings_for_route",
        "mark_invalid_for_tests",
    ] {
        assert!(
            !src.contains(deleted),
            "runtime.rs must not reference deleted A/B symbol {deleted}"
        );
    }
}

#[test]
fn t4_align_runtime_hook_chat_turn_create_turn_carry_request_ids() {
    with_sandbox(|| {
        let master = create_bound_plan("align-request-ids");
        install_cursor_engine();
        let log = install_fake_cursor_runtime();
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        let result = runtime::chat_turn(&sid, "align please", Some(&master)).expect("cursor");
        assert!(
            result
                .body["reply_text"]
                .as_str()
                .unwrap_or("")
                .contains("fake-ok"),
            "reply={}",
            result.body["reply_text"]
        );

        let entries = log.lock().unwrap().clone();
        for method in ["create", "turn"] {
            let hit = entries
                .iter()
                .find(|e| e.method == method)
                .unwrap_or_else(|| panic!("expected {method} via CSR hook: {entries:?}"));
            assert!(
                hit.request_id.is_some(),
                "{method} must be request-only (request_id set), got {hit:?}"
            );
        }
    });
}

#[test]
fn t4_ui_session_close_keeps_agent_slot_alive_without_dispose() {
    with_sandbox(|| {
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let factory_log = log.clone();
        let runtime_rt = Arc::new(CursorSessionRuntime::with_client_factory(move |_key| {
            let client = FakeCursorRunnerClient::from_shared(factory_log.clone());
            Ok(Box::new(client) as Box<dyn cursor_adapter::CursorRunnerClient>)
        }));

        runtime_rt
            .run_turn(&TurnRequest {
                session_id: "ui-close-session".into(),
                prompt: "warm".into(),
                api_key: "sk-test".into(),
                profile: super::test_business_profile("composer-2.5"),
            })
            .expect("create agent slot");
        runtime::set_cursor_runtime_for_tests(Some(runtime_rt.clone()));

        runtime::on_ui_session_close("ui-close-session");

        assert!(
            runtime_rt.has_session("ui-close-session"),
            "UI session close must leave the Agent slot alive"
        );
        let entries = log.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            !entries.iter().any(|entry| entry.method == "close"),
            "UI session close must not send Runner close/dispose: {entries:?}"
        );
    });
}
