//! T5: end-to-end contract / lifecycle / security / closing-gate tests.
//!
//! Closing-gate notes must distinguish contract-pass vs live-pass / pending
//! external acceptance. Mock success must NEVER mark Cursor AC satisfied.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings;
use crate::services::agent::cursor_adapter::{
    self, CursorErrorCode, CursorSessionRuntime, FakeCursorRunnerClient,
    HOST_GENERIC_UPSTREAM_UNAVAILABLE,
};
use crate::services::agent::e2e_gate::{
    self, ClosingGateReport, ContractVerdict, CursorAcVerdict, LiveSmokeVerdict,
};
use crate::services::agent::r#loop::{self, EVENT_TURN_COMPLETED};
use crate::services::agent::runtime;
use crate::services::agent::session_cwd;
use crate::services::mcp_server_registry::{self, SEEDED_BUSINESS_KEY};
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
    session_cwd::reset_for_tests();
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();
    e2e_gate::reset_for_tests();
    f();
    e2e_gate::reset_for_tests();
    runtime::reset_for_tests();
    r#loop::reset_runtime_for_tests();
    session_cwd::reset_for_tests();
    mcp_server_registry::clear_for_tests();
}

fn create_bound_plan(title: &str) -> String {
    let created = todo_task::create_master_with_subs(title, Some(&["子项A"]));
    assert_eq!(created["_status"], 201);
    created["master_task_id"].as_str().unwrap().to_string()
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
    secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-test-cursor-secret").expect("cursor key");
}

fn install_fake_cursor_runtime() -> Arc<Mutex<Vec<cursor_adapter::FakeLogEntry>>> {
    let fake = FakeCursorRunnerClient::new();
    let log = fake.log.clone();
    let runtime_rt = CursorSessionRuntime::with_client_factory(move |key| {
        let mut c = FakeCursorRunnerClient::from_shared(log.clone());
        c.on_spawned_with_api_key(key);
        Ok(Box::new(c) as Box<dyn cursor_adapter::CursorRunnerClient>)
    });
    runtime::set_cursor_runtime_for_tests(Some(Arc::new(runtime_rt)));
    runtime::set_profile_for_tests(Some(super::test_business_profile("composer-2.5")));
    fake.log.clone()
}

// ── Contract: Cursor route → runner; Profile cwd + MCP; Host isolation; secrets; errors ──

#[test]
fn t5_contract_cursor_route_calls_runner_with_cwd_and_ready_mcp() {
    with_sandbox(|| {
        let master = create_bound_plan("t5-contract-cursor");
        let mock = spawn_scripted_llm(vec![assistant_text("HOST-MUST-NOT-RUN")]);
        install_host_llm(&mock);
        install_cursor_engine();
        let log = install_fake_cursor_runtime();

        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        let result = runtime::chat_turn(&sid, "ping cursor", Some(&master)).expect("cursor");
        assert_eq!(mock.hits.lock().unwrap().len(), 0, "Host LLM client must not be called");
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap_or("")
                .contains("fake-ok"),
            "runner turn must surface"
        );

        let entries = log.lock().unwrap().clone();
        let create = entries
            .iter()
            .find(|e| e.method == "create")
            .expect("Cursor route must call runner create");
        let params = create.params.as_ref().expect("create params");
        assert!(
            params.get("apiKey").is_none() && params.get("api_key").is_none(),
            "credentials must never appear in IPC/JSONL: {params}"
        );
        let cwd = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .expect("local.cwd must be passed");
        let expected_cwd =
            session_cwd::session_cwd_for(&sid).expect("legacy test runtime session cwd");
        assert_eq!(Path::new(cwd), expected_cwd.as_path());
        assert!(
            Path::new(cwd).is_dir(),
            "legacy test runtime cwd must exist on disk: {cwd}"
        );
        assert!(
            !Path::new(cwd).join(".git").exists(),
            "legacy test runtime cwd must not be a git worktree root"
        );

        let mcp = params.get("mcpServers").expect("ready MCP must be passed");
        let expected_profile = super::test_business_profile("composer-2.5");
        let expected = serde_json::to_value(&expected_profile.mcp_servers).unwrap();
        assert_eq!(
            mcp, &expected,
            "health-checked HTTP MCP config must be injected"
        );

        let blob = result.body.to_string();
        assert!(
            !blob.contains("sk-test-cursor-secret") && !blob.contains("CURSOR_API_KEY"),
            "credentials must not leak into chat turn IPC body"
        );
        assert_ne!(
            result.body["reply_text"].as_str(),
            Some(HOST_GENERIC_UPSTREAM_UNAVAILABLE)
        );
    });
}

#[test]
fn t5_contract_host_route_does_not_start_node_child() {
    with_sandbox(|| {
        let master = create_bound_plan("t5-contract-host");
        let mock = spawn_scripted_llm(vec![assistant_text("host-ok")]);
        install_host_llm(&mock);
        let open = {
            let tools_binding = json!([
                { "name": "get_plan", "ctx": { "master_task_id": master } },
            ]);
            r#loop::set_binding(crate::services::agent::session::Binding {
                tools: tools_binding,
                prompt: json!(crate::services::agent::PLAN_ASSISTANT_SYSTEM_PROMPT),
                callbacks: json!({}),
            })
            .expect("bind");
            r#loop::open_ai_assistant_core(&master).unwrap()
        };
        let sid = open["session_id"].as_str().unwrap();
        let result = runtime::chat_turn(sid, "你好", Some(&master)).expect("host");
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap_or("")
                .contains("host-ok")
        );
        assert!(
            !runtime::cursor_runtime_installed_for_tests(),
            "Host route must not install/start Cursor Node child"
        );
        assert_eq!(
            session_cwd::session_cwd_for(sid),
            None,
            "Host must not allocate Cursor cwd"
        );
        assert!(result.emit_turn_completed.is_some());
        assert_eq!(
            result.emit_turn_completed.as_ref().unwrap()["event"],
            EVENT_TURN_COMPLETED
        );
    });
}

#[test]
fn t5_contract_errors_are_distinguishable_not_host_generic() {
    with_sandbox(|| {
        let master = create_bound_plan("t5-errors");
        install_cursor_engine();
        secrets::test_secrets_clear();
        let mut s = settings::load().expect("load");
        s.assistant_engine = "cursor".into();
        settings::save(&s).expect("save");

        runtime::set_cursor_runtime_for_tests(Some(Arc::new(
            CursorSessionRuntime::with_client_factory(|_| {
                panic!("must not spawn without credential")
            }),
        )));
        runtime::set_profile_for_tests(Some(super::test_business_profile("composer-2.5")));
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let cred = runtime::chat_turn(sid, "x", None).expect("envelope");
        assert_eq!(cred.body["terminal"], "error");
        assert_eq!(
            cred.body["reply_text"].as_str(),
            Some(cursor_adapter::frontend_message_for(CursorErrorCode::Credential))
        );
        assert_ne!(
            cred.body["reply_text"].as_str(),
            Some(HOST_GENERIC_UPSTREAM_UNAVAILABLE)
        );
        assert_eq!(cred.body["code"], "cursor_credential");

        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-x").expect("key");
        runtime::set_cursor_runtime_for_tests(Some(Arc::new(
            CursorSessionRuntime::with_client_factory(|_| {
                Err(cursor_adapter::CursorError::new(
                    CursorErrorCode::Runner,
                    cursor_adapter::frontend_message_for(CursorErrorCode::Runner),
                ))
            }),
        )));
        let runner = runtime::chat_turn(sid, "y", None).expect("envelope");
        assert_eq!(runner.body["code"], "cursor_runner");
        assert_ne!(
            runner.body["reply_text"],
            cred.body["reply_text"],
            "credential vs runner messages must differ"
        );
        assert_ne!(
            runner.body["reply_text"].as_str(),
            Some(HOST_GENERIC_UPSTREAM_UNAVAILABLE)
        );
    });
}

// ── Lifecycle / security ─────────────────────────────────────────────────────

#[test]
fn t5_lifecycle_reset_cancels_inflight_and_blocks_stale_persist() {
    with_sandbox(|| {
        let fake = FakeCursorRunnerClient::new();
        fake.set_turn_block(Duration::from_millis(300));
        let log = fake.log.clone();
        let cursor_rt = Arc::new(CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        }));

        let rt_a = cursor_rt.clone();
        let handle = std::thread::spawn(move || {
            rt_a.run_turn(&cursor_adapter::TurnRequest {
                session_id: "sess_t5_cut".into(),
                prompt: "in-flight".into(),
                api_key: "sk-test".into(),
                profile: super::test_business_profile("composer-2.5"),
            })
        });
        std::thread::sleep(Duration::from_millis(40));
        cursor_rt
            .cancel_for_binding_cut("sess_t5_cut")
            .expect("cancel");
        let turn = handle.join().expect("join");
        match turn {
            Ok(r) => assert!(
                !r.should_persist,
                "stale/cancelled results must not persist"
            ),
            Err(e) => {
                assert_eq!(e.code, CursorErrorCode::Cancelled);
                assert!(!cursor_adapter::should_persist_cursor_result(&Err(e)));
            }
        }
        let methods: Vec<_> = fake
            .log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(
            methods.iter().any(|m| m == "cancel"),
            "Reset/cut must cancel in-flight: {methods:?}"
        );
    });
}

#[test]
fn t5_lifecycle_close_awaits_dispose_then_deletes_cwd_shell_close_does_not() {
    with_sandbox(|| {
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let cursor_rt = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });
        cursor_rt
            .run_turn(&cursor_adapter::TurnRequest {
                session_id: "sess_t5_close".into(),
                prompt: "hi".into(),
                api_key: "sk-test".into(),
                profile: super::test_business_profile("composer-2.5"),
            })
            .expect("turn");
        let cwd = session_cwd::session_cwd_for("sess_t5_close").expect("cwd");
        assert!(cwd.is_dir());

        cursor_rt.on_shell_close("sess_t5_close");
        assert!(cwd.is_dir(), "shell close must not clean cwd");
        assert!(cursor_rt.has_session("sess_t5_close"));
        let after_shell: Vec<_> = fake
            .log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(!after_shell.iter().any(|m| m == "close"));

        cursor_rt.close_session("sess_t5_close").expect("close");
        assert!(
            fake.dispose_awaited.load(std::sync::atomic::Ordering::SeqCst),
            "close must await dispose before ok"
        );
        assert!(!cwd.exists(), "cwd deleted only after await-dispose close");
        assert!(!cursor_rt.has_session("sess_t5_close"));
    });
}

#[test]
fn t5_security_sandbox_fail_closed_and_no_tools_dispatch_on_cursor_path() {
    let sandbox_src = include_str!("../../../../../packages/cursor-agent-runner/src/sandbox.ts");
    let runner_src = include_str!("../../../../../packages/cursor-agent-runner/src/index.ts");
    assert!(
        sandbox_src.contains("ensureNodeAndSandbox"),
        "Node/sandbox gate helper must exist"
    );
    // Workbench A: Local headless + SDK sandbox blocks MCP interactive approval.
    assert!(
        runner_src.contains("sandboxOptions: { enabled: false }"),
        "production runner must keep SDK sandbox disabled for Workbench MCP"
    );
    assert!(
        runner_src.contains("sdkSandboxEnabled: false"),
        "production create path must skip sandbox helper fail-closed"
    );
    let adapter_src = include_str!("../../../services/agent/cursor_adapter.rs");
    let runtime_src = include_str!("../../../services/agent/runtime.rs");
    for (label, src) in [("cursor_adapter", adapter_src), ("runtime", runtime_src)] {
        assert!(
            !src.contains("tools::dispatch") && !src.contains("crate::services::agent::tools::"),
            "{label} Cursor path must not call tools::dispatch"
        );
    }
    assert!(
        !include_str!("../../../services/agent/tools.rs").contains("pub fn dispatch"),
        "tools::dispatch must stay removed"
    );
}

// ── Closing gate: contract vs live; mock ≠ Cursor AC ─────────────────────────

#[test]
fn t5_closing_gate_distinguishes_contract_pass_from_live_and_rejects_mock_ac() {
    with_sandbox(|| {
        e2e_gate::record_contract_pass(true);
        e2e_gate::record_live_smoke(LiveSmokeVerdict::SkippedNoCredentials);

        let report: ClosingGateReport = e2e_gate::evaluate_closing_gate();
        assert_eq!(report.contract, ContractVerdict::Pass);
        assert_eq!(report.live, LiveSmokeVerdict::SkippedNoCredentials);
        assert_eq!(
            report.cursor_ac,
            CursorAcVerdict::PendingExternalAcceptance,
            "skip/mock must not mark Cursor AC satisfied"
        );
        assert!(
            report.notes.contains("外部验收待完成") || report.notes.contains("pending external"),
            "notes must retain external-acceptance pending: {}",
            report.notes
        );
        assert!(
            report.notes.contains("contract") || report.notes.contains("合同"),
            "notes must mention contract status"
        );
        assert!(
            !report.marks_cursor_ac_from_mock,
            "mock MUST NOT mark Cursor AC satisfied"
        );

        e2e_gate::record_mock_evidence_only(true);
        let report2 = e2e_gate::evaluate_closing_gate();
        assert_eq!(
            report2.cursor_ac,
            CursorAcVerdict::PendingExternalAcceptance
        );
        assert!(!report2.marks_cursor_ac_from_mock);

        e2e_gate::record_live_smoke(LiveSmokeVerdict::Pass);
        e2e_gate::record_mock_evidence_only(false);
        let report3 = e2e_gate::evaluate_closing_gate();
        assert_eq!(report3.live, LiveSmokeVerdict::Pass);
        assert_eq!(report3.cursor_ac, CursorAcVerdict::SatisfiedByLive);
    });
}

#[test]
fn t5_closing_gate_checks_runner_install_route_health_fields() {
    with_sandbox(|| {
        e2e_gate::record_contract_pass(true);
        e2e_gate::record_runner_installable(true);
        e2e_gate::record_runner_startable(true);
        e2e_gate::record_cursor_route_reachable(true);
        e2e_gate::record_endpoint_health(true);
        e2e_gate::record_live_smoke(LiveSmokeVerdict::SkippedNoCredentials);

        let report = e2e_gate::evaluate_closing_gate();
        assert!(report.runner_installable);
        assert!(report.runner_startable);
        assert!(report.cursor_route_reachable);
        assert!(report.endpoint_health_ok);
        assert_eq!(report.contract, ContractVerdict::Pass);
        assert_eq!(
            report.cursor_ac,
            CursorAcVerdict::PendingExternalAcceptance
        );

        let md = e2e_gate::closing_gate_markdown(&report);
        assert!(
            md.contains("外部验收待完成") || md.contains("PendingExternalAcceptance"),
            "markdown must record pending external acceptance"
        );
        assert!(
            md.contains("contract")
                || md.contains("合同通过")
                || md.contains("ContractVerdict::Pass"),
            "markdown must distinguish contract pass"
        );
        assert!(
            !md.contains("Cursor AC: satisfied by mock")
                && !md.contains("Cursor AC 已满足（mock）"),
            "must never claim mock-satisfied Cursor AC"
        );
    });
}
