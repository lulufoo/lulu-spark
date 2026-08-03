//! T4: Cursor Local adapter — Agent SDK Local shape + mcpServers injection.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::services::agent::cursor_adapter::{self, AgentCreateParams, CursorAdapterError};
use crate::services::agent::engine_router::{self, AdapterKind, EngineKind, TurnInput};
use crate::services::agent::r#loop;
use crate::services::mcp_server_registry::{self, McpServerConfig, SEEDED_BUSINESS_KEY};
use crate::services::todo_task;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();
    r#loop::reset_runtime_for_tests();
    f();
    r#loop::reset_runtime_for_tests();
    mcp_server_registry::clear_for_tests();
}

fn key_only_payload(key: &str) -> serde_json::Value {
    json!({ "key": key })
}

fn sample_cwd() -> PathBuf {
    PathBuf::from("/tmp/cursor-adapter-t4-placeholder-cwd")
}

/// Capturing double for the replaceable Cursor Agent SDK Local port (A2).
#[derive(Default)]
struct CapturingSdk {
    creates: Vec<AgentCreateParams>,
    fail_create: bool,
    fail_run: bool,
}

impl cursor_adapter::CursorAgentSdk for CapturingSdk {
    fn create(&mut self, params: AgentCreateParams) -> Result<(), CursorAdapterError> {
        self.creates.push(params);
        if self.fail_create {
            Err(CursorAdapterError::Sdk("create failed".into()))
        } else {
            Ok(())
        }
    }

    fn run_turn(&mut self, _message: &str) -> Result<String, CursorAdapterError> {
        if self.fail_run {
            Err(CursorAdapterError::Sdk("run_turn failed".into()))
        } else {
            Ok("cursor-sdk-ok".into())
        }
    }
}

#[test]
fn t4_cursor_adapter_module_is_isolated_from_host_loop() {
    // Cursor adapter is a sibling module; Host loop/llm must not link it.
    let loop_src = include_str!("../../../services/agent/loop.rs");
    let llm_src = include_str!("../../../services/agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        assert!(
            !src.contains("cursor_adapter"),
            "{label} must not import cursor_adapter (Host path zero Cursor)"
        );
    }
    // Adapter itself must not call process-local tools::dispatch.
    let adapter_src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        !adapter_src.contains("tools::dispatch")
            && !adapter_src.contains("crate::services::agent::tools"),
        "cursor_adapter must not call tools::dispatch / import tools"
    );
    assert!(
        cursor_adapter::CURSOR_SDK_A2_REPLACEABLE_PORT,
        "A2 must be explicit: SDK embed via replaceable port (real crate unavailable)"
    );
}

#[test]
fn t4_mcp_config_maps_to_mcp_servers_inline_shape() {
    let config = McpServerConfig {
        capability_description: "internal knowledge-mcp todo_task capability surface".into(),
        http_transport: crate::services::mcp_server_registry::HttpMcpTransport {
            name: "workbench".into(),
            url: "http://127.0.0.1:9876/mcp".into(),
            headers: Default::default(),
        },
    };
    let servers = cursor_adapter::map_mcp_config_to_mcp_servers(&config);
    let v = serde_json::to_value(&servers).expect("serialize mcpServers");
    // Agent SDK Local inline mcpServers: named server entry with decision-level config.
    assert!(
        v.as_object().map(|o| !o.is_empty()).unwrap_or(false),
        "mcpServers must be a non-empty object: {v}"
    );
    let entry = v
        .as_object()
        .and_then(|o| o.values().next())
        .expect("at least one server entry");
    assert_eq!(
        entry.get("capability_description").and_then(|x| x.as_str()),
        Some(config.capability_description.as_str())
    );
}

#[test]
fn t4_create_agent_injects_mcp_servers_and_local_cwd_placeholder() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let mcp = r#loop::session_capability_mcp_config().expect("L2 read face");
        let cwd = sample_cwd();
        let mut sdk = CapturingSdk::default();

        cursor_adapter::create_agent(&mut sdk, &mcp, cwd.clone()).expect("create");

        assert_eq!(sdk.creates.len(), 1);
        let params = &sdk.creates[0];
        assert_eq!(params.local_cwd, cwd, "local.cwd injection point reserved for t5");
        let expected = cursor_adapter::map_mcp_config_to_mcp_servers(&mcp);
        assert_eq!(params.mcp_servers, expected);
    });
}

#[test]
fn t4_run_turn_reads_session_capability_mcp_config_and_captures_create_params() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let before = r#loop::session_capability_mcp_config().expect("loaded");
        let cwd = sample_cwd();
        let mut sdk = CapturingSdk::default();
        let input = TurnInput {
            session_id: "sess_t4".into(),
            message: "hello cursor".into(),
        };

        let body = cursor_adapter::run_turn(&mut sdk, &input, cwd.clone()).expect("run");
        assert_eq!(body, "cursor-sdk-ok");
        assert_eq!(sdk.creates.len(), 1);
        assert_eq!(sdk.creates[0].local_cwd, cwd);
        assert_eq!(
            sdk.creates[0].mcp_servers,
            cursor_adapter::map_mcp_config_to_mcp_servers(&before)
        );
        // Read face must remain unchanged (read-only consumption).
        let after = r#loop::session_capability_mcp_config().expect("still loaded");
        assert_eq!(after, before);
    });
}

#[test]
fn t4_settings_cursor_route_enters_cursor_adapter_module() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let cwd = sample_cwd();
        let sdk = Arc::new(Mutex::new(CapturingSdk::default()));
        let sdk_host = sdk.clone();
        let input = TurnInput {
            session_id: "sess_route_t4".into(),
            message: "route me".into(),
        };

        let out = engine_router::route_chat_turn(
            EngineKind::Cursor,
            &input,
            || Ok("host-must-not-run".into()),
            || {
                let mut guard = sdk_host.lock().expect("sdk lock");
                cursor_adapter::run_turn(&mut *guard, &input, cwd.clone())
                    .map_err(|e| e.to_string())
            },
        )
        .expect("cursor route");

        assert_eq!(out.adapter, AdapterKind::Cursor);
        assert_eq!(out.body, "cursor-sdk-ok");
        assert_eq!(sdk.lock().unwrap().creates.len(), 1);
    });
}

#[test]
fn t4_failure_path_does_not_call_tools_dispatch() {
    with_sandbox(|| {
        let created = todo_task::create_master_with_subs("t4-no-dispatch", Some(&["sub"]));
        assert_eq!(created["_status"], 201);
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let mut sdk = CapturingSdk {
            fail_create: true,
            ..CapturingSdk::default()
        };
        let input = TurnInput {
            session_id: "sess_fail".into(),
            message: "force fail".into(),
        };
        let err = cursor_adapter::run_turn(&mut sdk, &input, sample_cwd())
            .expect_err("SDK create failure must surface");
        assert!(
            matches!(err, CursorAdapterError::Sdk(_)),
            "expected Sdk error, got {err:?}"
        );
        // Cursor failure must not fall back to process-local tools (L09-I #7).
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
    });
}

#[test]
fn t4_missing_session_mcp_config_fails_without_tools_dispatch() {
    with_sandbox(|| {
        // No Binding Set → read face empty.
        assert!(r#loop::session_capability_mcp_config().is_none());
        let created = todo_task::create_master_with_subs("t4-no-mcp", Some(&["s"]));
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        let mut sdk = CapturingSdk::default();
        let input = TurnInput {
            session_id: "sess_no_mcp".into(),
            message: "no config".into(),
        };
        let err = cursor_adapter::run_turn(&mut sdk, &input, sample_cwd())
            .expect_err("missing MCP config must fail");
        assert!(
            matches!(err, CursorAdapterError::MissingMcpConfig),
            "expected MissingMcpConfig, got {err:?}"
        );
        assert!(
            sdk.creates.is_empty(),
            "must not Agent.create without mcpServers"
        );
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
    });
}

#[test]
fn t4_run_turn_sdk_failure_does_not_fallback_to_process_tools() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let created = todo_task::create_master_with_subs("t4-sdk-run-fail", Some(&["s"]));
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        let mut sdk = CapturingSdk {
            fail_run: true,
            ..CapturingSdk::default()
        };
        let input = TurnInput {
            session_id: "sess_run_fail".into(),
            message: "run fail".into(),
        };
        let err = cursor_adapter::run_turn(&mut sdk, &input, sample_cwd())
            .expect_err("run_turn SDK failure");
        assert!(matches!(err, CursorAdapterError::Sdk(_)));
        assert_eq!(sdk.creates.len(), 1, "create may succeed before run fails");
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
    });
}
