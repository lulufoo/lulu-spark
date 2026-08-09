//! T5 / T-ReplaceCreate: Host cleanup_session_cwd after replaced_session_id;
//! dispose-fail must not clean cwd; cwd cleanup must not touch Session cache.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    CursorError, CursorErrorCode, CursorLlmEngine, CursorRunnerClient, FakeLogEntry, TurnRequest,
};
use crate::services::agent::engine_router::{self, EngineRuntimeConfig};
use crate::services::agent::process_manager::{self, CursorAgentProcessManager};
use crate::services::agent::session;
use crate::services::agent::session_cwd::{self, CwdError};
use crate::services::mcp_endpoint_readiness::ReadyMcpTransports;
use crate::services::mcp_server_registry::HttpMcpTransport;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    process_manager::reset_global_for_tests();
    session_cwd::reset_for_tests();
    f();
    session_cwd::reset_for_tests();
    process_manager::reset_global_for_tests();
    secrets::test_secrets_clear();
}

fn settings_cursor() -> AppSettings {
    let mut s = AppSettings::default();
    s.assistant_engine = "cursor".into();
    settings::upsert_llm_entry(
        &mut s.llm,
        "cursor",
        &LlmSettings {
            model: "composer-1".into(),
            ..Default::default()
        },
    )
    .expect("upsert cursor");
    s
}

fn cfg_from(settings: &AppSettings) -> EngineRuntimeConfig {
    engine_router::read_engine_runtime_config(settings).expect("runtime config")
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

fn turn_req(session_id: &str, prompt: &str) -> TurnRequest {
    TurnRequest {
        session_id: session_id.into(),
        prompt: prompt.into(),
        model: "composer-1".into(),
        api_key: "sk-engine-test".into(),
        ready_mcp: ready_mcp_sample(),
    }
}

/// Test double: create returns configurable result / error; records params.
struct ReplaceCreateFake {
    log: Arc<Mutex<Vec<FakeLogEntry>>>,
    next_create_ok: Arc<Mutex<Option<Value>>>,
    next_create_err: Arc<Mutex<Option<(String, String)>>>,
    create_calls: Arc<AtomicUsize>,
}

impl ReplaceCreateFake {
    fn new() -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
            next_create_ok: Arc::new(Mutex::new(None)),
            next_create_err: Arc::new(Mutex::new(None)),
            create_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn clone_factory(
        &self,
    ) -> impl FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send {
        let log = self.log.clone();
        let next_ok = self.next_create_ok.clone();
        let next_err = self.next_create_err.clone();
        let create_calls = self.create_calls.clone();
        move |_key: &str| {
            Ok(Box::new(ReplaceCreateFakeClient {
                log: log.clone(),
                next_create_ok: next_ok.clone(),
                next_create_err: next_err.clone(),
                create_calls: create_calls.clone(),
            }) as Box<dyn CursorRunnerClient>)
        }
    }
}

struct ReplaceCreateFakeClient {
    log: Arc<Mutex<Vec<FakeLogEntry>>>,
    next_create_ok: Arc<Mutex<Option<Value>>>,
    next_create_err: Arc<Mutex<Option<(String, String)>>>,
    create_calls: Arc<AtomicUsize>,
}

impl CursorRunnerClient for ReplaceCreateFakeClient {
    fn create(
        &mut self,
        model: &str,
        cwd: &Path,
        mcp_servers: &Value,
    ) -> Result<(), CursorError> {
        let _ = self.request(
            "legacy",
            "create",
            Some(json!({
                "model": model,
                "cwd": cwd.to_string_lossy(),
                "mcpServers": mcp_servers,
            })),
        )?;
        Ok(())
    }

    fn turn(&mut self, prompt: &str) -> Result<String, CursorError> {
        let v = self.request("legacy", "turn", Some(json!({ "prompt": prompt })))?;
        Ok(v.get("text").and_then(|x| x.as_str()).unwrap_or("").into())
    }

    fn cancel(&mut self) -> Result<(), CursorError> {
        let _ = self.request("legacy", "cancel", None)?;
        Ok(())
    }

    fn close(&mut self) -> Result<(), CursorError> {
        let _ = self.request("legacy", "close", None)?;
        Ok(())
    }

    fn force_kill(&mut self) {}

    fn terminate_hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        Arc::new(|| {})
    }

    fn request(
        &mut self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        self.log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(FakeLogEntry {
                request_id: Some(request_id.into()),
                method: method.into(),
                params: params.clone(),
            });
        match method {
            "create" => {
                self.create_calls.fetch_add(1, Ordering::SeqCst);
                if let Some((ty, msg)) = self
                    .next_create_err
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                {
                    return Err(CursorError::new(
                        match ty.as_str() {
                            "cancelled" => CursorErrorCode::Cancelled,
                            "cwd" => CursorErrorCode::Cwd,
                            "credential" => CursorErrorCode::Credential,
                            _ => CursorErrorCode::Runner,
                        },
                        msg,
                    ));
                }
                if let Some(v) = self
                    .next_create_ok
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                {
                    return Ok(v);
                }
                Ok(json!({ "agentId": "fake" }))
            }
            "turn" => Ok(json!({ "text": "fake-ok" })),
            "cancel" => Ok(json!({ "cancelled": true })),
            "close" => Ok(json!({ "closed": true })),
            _ => Err(CursorError::new(
                CursorErrorCode::Runner,
                format!("unknown method {method}"),
            )),
        }
    }
}

#[test]
fn t5_create_params_include_opaque_session_id() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = ReplaceCreateFake::new();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            fake.clone_factory(),
        );
        let engine = CursorLlmEngine::with_manager(mgr);
        engine
            .run_turn(&turn_req("sess_wire_id", "hi"))
            .expect("turn");

        let log = fake.log.lock().unwrap_or_else(|e| e.into_inner());
        let create = log
            .iter()
            .find(|e| e.method == "create")
            .expect("create logged");
        let params = create.params.as_ref().expect("params");
        assert_eq!(
            params.get("session_id").and_then(|x| x.as_str()),
            Some("sess_wire_id"),
            "create must carry opaque session_id: {params}"
        );
    });
}

#[test]
fn t5_host_cleanup_cwd_only_after_replaced_session_id() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = ReplaceCreateFake::new();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            fake.clone_factory(),
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        engine
            .run_turn(&turn_req("sess_old", "a"))
            .expect("old turn");
        let old_cwd = session_cwd::session_cwd_for("sess_old").expect("old cwd");
        assert!(old_cwd.is_dir());

        *fake
            .next_create_ok
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(json!({
            "agentId": "agent-new",
            "replaced_session_id": "sess_old",
        }));

        engine
            .run_turn(&turn_req("sess_new", "b"))
            .expect("new turn");

        assert!(
            !old_cwd.exists(),
            "Host must cleanup_session_cwd(replaced_session_id) after successful replace"
        );
        assert!(
            session_cwd::session_cwd_for("sess_old").is_none(),
            "old allocation cleared"
        );
        let new_cwd = session_cwd::session_cwd_for("sess_new").expect("new cwd");
        assert!(new_cwd.is_dir());
        assert_ne!(old_cwd, new_cwd);
    });
}

#[test]
fn t5_dispose_fail_create_error_skips_cwd_cleanup_and_replaced_id() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = ReplaceCreateFake::new();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            fake.clone_factory(),
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        engine
            .run_turn(&turn_req("sess_keep", "a"))
            .expect("first");
        let keep = session_cwd::session_cwd_for("sess_keep").expect("cwd");
        assert!(keep.is_dir());

        *fake
            .next_create_err
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            Some(("runner".into(), "asyncDispose failed".into()));

        let err = engine
            .run_turn(&turn_req("sess_next", "b"))
            .expect_err("dispose-fail create must fail");
        assert_eq!(err.code, CursorErrorCode::Runner);

        assert!(
            keep.exists(),
            "Host must not cleanup old cwd when create omits replaced_session_id / fails"
        );
        assert!(session_cwd::session_cwd_for("sess_keep").is_some());
    });
}

#[test]
fn t5_cwd_cleanup_does_not_delete_business_session_cache() {
    with_sandbox(|| {
        let sess = session::create_session(Some("task_x"), Some("title")).expect("session");
        let sid = sess.session_id.clone();
        let cwd = session_cwd::create_session_cwd(&sid).expect("cwd");
        assert!(cwd.is_dir());
        let cache_path = session::session_file_path(&sid).expect("cache path");
        assert!(cache_path.is_file());

        session_cwd::cleanup_session_cwd(&sid).expect("cleanup cwd");
        assert!(!cwd.exists(), "cwd removed");
        assert!(
            cache_path.is_file(),
            "business Session cache must survive cwd cleanup"
        );
        let loaded = session::load_session(&sid).expect("reload");
        assert_eq!(loaded.session_id, sid);
    });
}

#[test]
fn t5_cwd_cleanup_failure_retains_dir_as_recoverable_error() {
    with_sandbox(|| {
        let path = session_cwd::create_session_cwd("sess_retain").expect("create");
        session_cwd::force_cleanup_fail_for_tests(true);
        let err = session_cwd::cleanup_session_cwd("sess_retain").expect_err("retain");
        match err {
            CwdError::CleanupRetained(msg) => assert!(!msg.is_empty()),
            other => panic!("expected CleanupRetained, got {other:?}"),
        }
        assert!(path.exists(), "retain isolation dir on cleanup failure");
        session_cwd::force_cleanup_fail_for_tests(false);
    });
}

#[test]
fn t5_shell_close_without_binding_replace_is_noop_for_agent() {
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        adapter.contains("fn on_shell_close"),
        "shell-close hook must exist"
    );
    let idx = adapter
        .find("pub fn on_shell_close")
        .expect("on_shell_close");
    let window = &adapter[idx..idx + 200.min(adapter.len() - idx)];
    assert!(
        !window.contains("cleanup_session_cwd") && !window.contains("\"close\""),
        "shell-close without binding replace must not dispose/cleanup: {window}"
    );
}
