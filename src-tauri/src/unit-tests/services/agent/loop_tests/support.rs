//! Shared Loop test fixtures (sandbox, mock LLM, Binding helpers).

pub(super) use std::collections::BTreeMap;
pub(super) use std::fs;
pub(super) use std::io::{Read, Write};
pub(super) use std::net::TcpListener;
pub(super) use std::sync::{Arc, Mutex};
pub(super) use std::thread;
pub(super) use std::time::Duration;

pub(super) use serde_json::{json, Value};

pub(super) use crate::config::secrets::{self, KEY_LLM_API_KEY};
pub(super) use crate::config::settings;
pub(super) use crate::services::agent::llm::LlmConfig;
pub(super) use crate::services::agent::r#loop::{self, Terminal, TurnOutcome, EVENT_TURN_COMPLETED};
pub(super) use crate::services::agent::session::{self, Turn};
pub(super) use crate::services::agent::PLAN_ASSISTANT_SYSTEM_PROMPT;
pub(super) use crate::services::local_http;
pub(super) use crate::services::mcp_oauth::{
    issue_for_slot, ledger_record, revoke_for_slot, test_force_keychain_unavailable,
    verify_for_slot, Slot, TicketHandle, TicketState,
};
pub(super) use crate::services::mcp_host::{
    start_embedded_mcp_runtime_with_sidecar, stop_embedded_mcp_runtime, McpRuntimeConfig,
};
pub(super) use crate::services::mcp_host::registry::{HttpMcpTransport, McpServerConfig};
pub(super) use crate::services::mcp_host::registry as mcp_registry;
pub(super) use crate::services::todo_task;
pub(super) use crate::test_support::TestSandbox;

pub(super) use super::super::loop_src::LOOP_SRC;

pub(super) fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    test_force_keychain_unavailable(false);
    crate::services::mcp_host::registry::clear_for_tests();
    crate::services::mcp_host::registry::seed_defaults();
    f();
}

pub(super) fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("ephemeral address")
        .port()
}

pub(super) fn start_isolated_mcp(
    sandbox: &TestSandbox,
) -> (local_http::LocalHttpHandle, crate::services::mcp_host::McpRuntimeHandle, u16) {
    let sidecar_port = ephemeral_port();
    let sidecar = local_http::start(sandbox.config_dir().to_path_buf(), sidecar_port)
        .expect("start isolated Sidecar");
    let mcp_port = ephemeral_port();
    let mcp = start_embedded_mcp_runtime_with_sidecar(
        McpRuntimeConfig {
            bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("MCP address"),
        },
        format!("http://127.0.0.1:{sidecar_port}"),
    )
    .expect("start isolated MCP");
    (sidecar, mcp, mcp_port)
}

pub(super) fn bearer_headers_for_slot(scene: &str) -> BTreeMap<String, String> {
    let ticket = issue_for_slot(Slot::parse(scene).expect("registered slot")).expect("issue ticket");
    BTreeMap::from([(
        "Authorization".to_string(),
        format!("Bearer {}", ticket.as_str()),
    )])
}

pub(super) fn register_test_mcp(scene: &str, mcp_port: u16) {
    mcp_registry::register(
        scene,
        McpServerConfig {
            capability_description: format!("{scene} test capability"),
            http_transport: HttpMcpTransport {
                name: format!("{scene}-test"),
                url: format!("http://127.0.0.1:{mcp_port}/mcp/{scene}"),
                headers: bearer_headers_for_slot(scene),
            },
        },
    )
    .expect("register scene MCP");
}

pub(super) fn create_bound_plan(title: &str) -> String {
    let created = todo_task::create_master_with_subs(title, Some(&["子项A"])).expect("todo");
    created["master_task_id"].as_str().unwrap().to_string()
}


pub(super) fn plan_tools_binding(master: &str) -> r#loop::Binding {
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

pub(super) fn arm_plan_binding(master: &str) {
    r#loop::set_binding(plan_tools_binding(master)).expect("Set plan Binding");
}


pub(super) struct MockLlm {
    pub(super) port: u16,
    pub(super) hits: Arc<Mutex<Vec<Value>>>,
    pub(super) _join: thread::JoinHandle<()>,
}

pub(super) fn spawn_scripted_llm(responses: Vec<(u16, Value)>) -> MockLlm {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_t = hits.clone();
    let join = thread::spawn(move || {
        let mut idx = 0usize;
        for _ in 0..responses.len().saturating_add(4) {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = [0u8; 65536];
            let n = stream.read(&mut buf).unwrap_or(0);
            let raw = String::from_utf8_lossy(&buf[..n]);
            let body_str = raw.split("\r\n\r\n").nth(1).unwrap_or("");
            let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
                .unwrap_or(json!({}));
            hits_t.lock().unwrap().push(body);
            let (status, resp) = if idx < responses.len() {
                responses[idx].clone()
            } else {
                (
                    200,
                    json!({
                        "choices": [{
                            "finish_reason": "stop",
                            "message": { "role": "assistant", "content": "ok" }
                        }]
                    }),
                )
            };
            idx += 1;
            let body = resp.to_string();
            let resp = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
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

pub(super) fn cfg_for(mock: &MockLlm) -> LlmConfig {
    LlmConfig {
        api_key: "sk-test".into(),
        base_url: format!("http://127.0.0.1:{}", mock.port),
        model: "test-model".into(),
    }
}

pub(super) fn install_llm_cfg(mock: &MockLlm) {
    let mut s = settings::load().expect("load");
    settings::apply_config_payload(
        &mut s,
        &json!({
            "assistant_engine": "host",
            "llm": {
                "model": "test-model"
            }
        }),
    )
    .expect("apply");
    // Preset platform/base_url are not client-writable via apply_config_payload;
    // persist mock URL on the settings struct for Host load_llm_config (same pattern as
    // unit-tests/services/agent/mod.rs::llm_load_config_reads_settings_and_secret).
    let model = settings::llm_entry_by_type(&s.llm, "host")
        .map(|e| e.model.clone())
        .unwrap_or_else(|| "test-model".into());
    settings::upsert_llm_entry(
        &mut s.llm,
        "host",
        &settings::LlmSettings {
            platform: "glm".into(),
            base_url: format!("http://127.0.0.1:{}", mock.port),
            model,
        },
    )
    .expect("upsert host llm");
    settings::save(&s).expect("save");
    secrets::set_secret(KEY_LLM_API_KEY, "sk-test").expect("key");
}

pub(super) fn assistant_text(content: &str) -> (u16, Value) {
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

pub(super) fn assistant_tools(calls: Value, content: Option<&str>) -> (u16, Value) {
    (
        200,
        json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "content": content,
                    "tool_calls": calls
                }
            }]
        }),
    )
}

/// Typed/internal bindings without an MCP capability remain text-only.
pub(super) fn assert_host_llm_tools_empty(body: &Value) {
    match body.get("tools") {
        None => {}
        Some(Value::Array(arr)) => {
            assert!(
                arr.is_empty(),
                "Host path must send tools=[] (empty), got {arr:?}"
            );
        }
        Some(other) => panic!("Host path tools must be absent or [], got {other}"),
    }
    if let Some(tc) = body.get("tool_choice") {
        // Text-only typed bindings must not request a tool choice.
        panic!("typed text-only path must not send tool_choice, got {tc}");
    }
}

pub(super) fn assert_outcome(o: &TurnOutcome, terminal: &str, wrote: bool) {
    assert_eq!(o.terminal.as_str(), terminal, "terminal mismatch: {o:?}");
    assert_eq!(o.wrote, wrote, "wrote mismatch: {o:?}");
    assert!(!o.reply_text.is_empty(), "reply_text empty: {o:?}");
}

// --- Binding Contract B1: tools + prompt + callbacks Set validation (t1) ---

pub(super) fn applicable_tools() -> Value {
    json!([{ "name": "tool_a", "handle": "opaque-tool-a" }])
}

pub(super) fn applicable_prompt() -> Value {
    json!("opaque-system-prompt")
}

pub(super) fn empty_callbacks_registry() -> Value {
    json!({})
}

pub(super) fn valid_binding() -> r#loop::Binding {
    r#loop::Binding {
        tools: applicable_tools(),
        prompt: applicable_prompt(),
        callbacks: empty_callbacks_registry(),
    }
}

// --- Binding Contract state machine: Set / Reset / query + serialization (t2) ---

pub(super) fn binding_with_prompt(prompt: &str) -> r#loop::Binding {
    r#loop::Binding {
        tools: applicable_tools(),
        prompt: json!(prompt),
        callbacks: empty_callbacks_registry(),
    }
}

pub(super) fn assert_query_unbound(summary: &r#loop::BindingStateSummary) {
    assert_eq!(summary.state, "unbound");
    assert!(
        summary.generation.is_none(),
        "unbound query must not retain a live binding generation: {summary:?}"
    );
}

pub(super) fn assert_query_bound(summary: &r#loop::BindingStateSummary) {
    assert_eq!(summary.state, "bound");
    assert!(
        summary.generation.is_some(),
        "bound query must expose a generation for discard/replace proofs: {summary:?}"
    );
}

pub(super) fn assert_query_is_business_agnostic(summary: &r#loop::BindingStateSummary) {
    let encoded = serde_json::to_value(summary).expect("encode BindingStateSummary");
    let obj = encoded
        .as_object()
        .expect("BindingStateSummary encodes as object");
    assert!(
        !obj.contains_key("tools"),
        "query must not return tools slot: {encoded}"
    );
    assert!(
        !obj.contains_key("prompt"),
        "query must not return prompt slot: {encoded}"
    );
    assert!(
        !obj.contains_key("callbacks"),
        "query must not return callbacks slot: {encoded}"
    );
    assert!(
        !obj.contains_key("master_task_id"),
        "query must not return business id master_task_id: {encoded}"
    );
    assert!(
        !obj.contains_key("bound_master_task_id"),
        "query must not return business id bound_master_task_id: {encoded}"
    );
    let blob = encoded.to_string();
    assert!(
        !blob.contains("opaque-system-prompt")
            && !blob.contains("opaque-tool-a")
            && !blob.contains("secret-prompt-body")
            && !blob.contains("task_business"),
        "query summary must not leak tools/prompt body or business ids: {blob}"
    );
}

// --- execute gate + mid-execute Reset (strategy A) + lifecycle callbacks (t3) ---

pub(super) fn event_names(events: &[r#loop::LifecycleEvent]) -> Vec<&str> {
    events.iter().map(|e| e.event).collect()
}

pub(super) fn assert_payload_contract_only(ev: &r#loop::LifecycleEvent) {
    let encoded = serde_json::to_value(ev).expect("encode LifecycleEvent");
    let obj = encoded
        .as_object()
        .expect("LifecycleEvent encodes as object");
    assert!(obj.contains_key("event"), "event name required: {encoded}");
    // category may be absent for onBound/onUnbound
    for forbidden in [
        "tools",
        "prompt",
        "callbacks",
        "master_task_id",
        "bound_master_task_id",
        "binding",
        "generation",
        "applied_tools",
        "applied_prompt",
    ] {
        assert!(
            !obj.contains_key(forbidden),
            "callback payload must not carry {forbidden}: {encoded}"
        );
    }
    let blob = encoded.to_string();
    assert!(
        !blob.contains("opaque-system-prompt")
            && !blob.contains("opaque-tool-a")
            && !blob.contains("secret-prompt")
            && !blob.contains("task_business"),
        "callback payload must not leak tools/prompt body or business ids: {blob}"
    );
}

// --- J1 / execute 门闩内核验收夹具（SK-4 / H1：无业务 UI 驱动）---
// 通用 Binding 夹具（tools+prompt+callbacks；callbacks 可空表）驱动全部 J1 场景。

pub(super) fn j1_generic_binding_fixture(prompt: &str) -> r#loop::Binding {
    // Explicit generic fixture — no business page / 角位 / window-click driver.
    r#loop::Binding {
        tools: json!([{ "name": "fixture_tool", "handle": "opaque-fixture-tool" }]),
        prompt: json!(prompt),
        callbacks: json!({}), // empty registry allowed when slot present
    }
}

// --- T1: production path binding-generation gate ---

/// Mock LLM that runs `mid` (e.g. Reset / replace Set) before returning a scripted response.
pub(super) fn spawn_llm_with_mid_then_response<F>(mid: F, response: (u16, Value)) -> MockLlm
where
    F: FnOnce() + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_t = hits.clone();
    let mid = Arc::new(Mutex::new(Some(mid)));
    let join = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 65536];
        let n = stream.read(&mut buf).unwrap_or(0);
        let raw = String::from_utf8_lossy(&buf[..n]);
        let body_str = raw.split("\r\n\r\n").nth(1).unwrap_or("");
        let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
            .unwrap_or(json!({}));
        hits_t.lock().unwrap().push(body);
        if let Some(f) = mid.lock().unwrap().take() {
            f();
        }
        let (status, resp) = response;
        let body = resp.to_string();
        let resp = format!(
            "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes());
    });
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

// --- T2: session cut — clear-first (current_session_id → None) ---

pub(super) fn live_session_id() -> Option<String> {
    let sid = r#loop::get_ai_assistant_binding_core()["session_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if sid.is_empty() {
        None
    } else {
        Some(sid)
    }
}

// --- T3: symmetric in-flight cancel for execute + chat ---

pub(super) fn session_turn_contents(sid: &str) -> Vec<Option<String>> {
    session::load_session(sid)
        .unwrap()
        .turns
        .iter()
        .map(|t| t.content.clone())
        .collect()
}

pub(super) fn record_session_calls() -> (
    Arc<Mutex<Vec<String>>>,
    impl FnMut(&str) -> Result<(), String>,
    impl FnMut(&str, &str) -> Result<r#loop::ChatTurnResult, String>,
) {
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let create_calls = calls.clone();
    let turn_calls = calls.clone();
    let create = move |sid: &str| {
        create_calls.lock().unwrap().push(format!("create:{sid}"));
        Ok(())
    };
    let turn = move |sid: &str, prompt: &str| {
        turn_calls
            .lock()
            .unwrap()
            .push(format!("turn:{sid}:{prompt}"));
        Ok(r#loop::ChatTurnResult {
            body: json!({ "ok": true }),
            emit_turn_completed: None,
        })
    };
    (calls, create, turn)
}

pub(super) fn switch_session_without_resetting_binding(label: &str) -> (String, String, u64) {
    let session_a = r#loop::ensure_chat_session_core()
        .expect("ensure A")
        .get("session_id")
        .and_then(Value::as_str)
        .expect("session A")
        .to_string();
    let generation = r#loop::query_binding().generation.expect("bound generation");
    let master = create_bound_plan(label);
    let session_b = r#loop::open_ai_assistant_core(&master)
        .expect("open B")
        .get("session_id")
        .and_then(Value::as_str)
        .expect("session B")
        .to_string();
    assert_ne!(session_a, session_b);
    assert_eq!(r#loop::binding_state(), "bound");
    assert_eq!(
        r#loop::query_binding().generation,
        Some(generation),
        "switching session must not re-Set Binding"
    );
    assert_eq!(
        session::live_context_owner()
            .current_business_id()
            .as_deref(),
        Some(crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY),
        "same Binding must keep the same capability"
    );
    assert_eq!(
        session::live_context_owner()
            .current_session_id()
            .as_deref(),
        Some(session_b.as_str()),
        "live holds only the current session identity"
    );
    (session_a, session_b, generation)
}

// --- t2: key-only Binding Set/Reset + MCP session capability context ---

pub(super) fn key_only_payload(key: &str) -> Value {
    json!({ "key": key })
}

pub(super) fn empty_tools_binding() -> r#loop::Binding {
    r#loop::Binding {
        tools: json!([]),
        prompt: json!(PLAN_ASSISTANT_SYSTEM_PROMPT),
        callbacks: json!({}),
    }
}

// --- t4: Host Binding key is workbench; old App keys fail at registry lookup ---
//
// Frontend consumer is process-level setWorkbenchBinding; pages no longer Set/Reset.

pub(super) fn repo_file(rel: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

pub(super) fn function_slice<'a>(src: &'a str, marker: &str) -> &'a str {
    let start = src.find(marker).unwrap_or(src.len());
    let rest = &src[start..];
    let end = rest.len().min(2000);
    &rest[..end]
}

pub(super) fn session_authorization_bearer() -> Option<String> {
    r#loop::loaded_mcp_server().and_then(|cfg| {
        cfg.http_transport()
            .headers
            .get("Authorization")
            .cloned()
    })
}

pub(super) fn reset_workbench_slot() {
    test_force_keychain_unavailable(false);
    revoke_for_slot(Slot::Workbench).expect("revoke workbench");
}

pub(super) fn assert_secret_absent_from(text: &str, secret: &str) {
    if secret.is_empty() {
        panic!("ticket secret must be non-empty");
    }
    if text.contains(secret) {
        panic!("secret must not appear in logs, errors, or formatted output");
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("authorization:")
        && !text.contains("[REDACTED]")
        && lower.contains("bearer ")
    {
        panic!("full Authorization must not appear in formatted output");
    }
}

pub(super) fn assert_workbench_session_holds_live_ticket_seed_untouched() {
    let loaded = r#loop::loaded_mcp_server().expect("session MCP transport");
    let seed = mcp_registry::lookup(mcp_registry::SEEDED_BUSINESS_KEY)
        .expect("registry seed");
    assert_eq!(
        loaded.capability_description, seed.capability_description,
        "decision-level capability must still match the registry seed"
    );
    assert_eq!(
        loaded.http_transport.url, seed.http_transport.url,
        "session transport url must stay the seed url"
    );
    assert_eq!(
        loaded.http_transport.name, seed.http_transport.name,
        "session transport name must stay the seed name"
    );
    assert!(
        !seed.http_transport.headers.contains_key("Authorization"),
        "lookup(workbench) seed headers must not carry Authorization"
    );
    let auth = loaded
        .http_transport
        .headers
        .get("Authorization")
        .expect("session transport copy must carry Authorization");
    let handle = auth
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty() && !value.contains(' '))
        .expect("session Authorization must be Bearer <handle>");
    verify_for_slot(Slot::Workbench, TicketHandle::from_secret(handle))
        .expect("session ticket must be the Live workbench ticket");
    let record = ledger_record(Slot::Workbench)
        .expect("ledger")
        .expect("workbench ledger row");
    assert_eq!(record.state, TicketState::Live);
    assert_eq!(record.handle, TicketHandle::from_secret(handle));
}

pub(super) fn assert_bound_key(key: &str) {
    assert_eq!(r#loop::binding_state(), "bound", "expected bound for {key}");
    let loaded = r#loop::loaded_mcp_server().expect("key-only Set must load MCP");
    assert!(
        loaded.http_transport().url.contains(&format!("/mcp/{key}")),
        "loaded MCP url must contain /mcp/{key}, got {}",
        loaded.http_transport().url
    );
    assert_eq!(
        session::live_context_owner().current_business_id().as_deref(),
        Some(key),
        "live business id must be {key}"
    );
    if key == mcp_registry::SEEDED_BUSINESS_KEY {
        assert_workbench_session_holds_live_ticket_seed_untouched();
    }
}

// --- t6: Reset unloads session ticket header; does not revoke workbench ledger ---

pub(super) fn rust_pub_item<'a>(src: &'a str, marker: &str) -> &'a str {
    let start = src.find(marker).unwrap_or_else(|| panic!("missing {marker}"));
    let rest = &src[start..];
    let end = rest[marker.len()..]
        .find("\npub ")
        .map(|i| marker.len() + i)
        .unwrap_or(rest.len());
    &rest[..end]
}

pub(super) fn session_ticket_handle() -> TicketHandle {
    let auth = session_authorization_bearer().expect("session ticket header");
    let secret = auth
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty() && !value.contains(' '))
        .expect("session Authorization must be Bearer <handle>");
    TicketHandle::from_secret(secret)
}

pub(super) fn live_workbench_record() -> crate::services::mcp_oauth::LedgerRecord {
    ledger_record(Slot::Workbench)
        .expect("ledger")
        .expect("workbench ledger row")
}

pub(super) fn assert_session_unloaded_without_ticket_header() {
    assert_eq!(r#loop::binding_state(), "unbound");
    let loaded = r#loop::loaded_mcp_server();
    assert!(
        loaded.is_none()
            || loaded
                .as_ref()
                .is_some_and(|cfg| !cfg.http_transport().headers.contains_key("Authorization")),
        "Reset must unload session config or strip the ticket header"
    );
    assert!(
        session_authorization_bearer().is_none(),
        "Reset must leave no session Authorization header"
    );
}
