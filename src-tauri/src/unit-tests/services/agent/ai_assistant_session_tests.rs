//! T-SessionMigrate / t4: binding / MCP / live session authority → AIAssistantSession.

use serde_json::json;

use crate::services::agent::r#loop::{self, Binding};
use crate::services::agent::session::{self, AIAssistantSession};
use crate::services::mcp_server_registry::{self, SEEDED_BUSINESS_KEY};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();
    f();
}

fn runtime_struct_body(src: &str) -> &str {
    let start = src
        .find("struct Runtime {")
        .expect("loop.rs must still declare struct Runtime");
    let after = &src[start..];
    let end = after.find("\n}").expect("Runtime struct closing brace");
    &after[..end]
}

// ── Surface / ownership 合同 ─────────────────────────────────────────────────

#[test]
fn t4_ai_assistant_session_type_lives_in_session_rs() {
    let _ = std::any::type_name::<AIAssistantSession>();
    let src = include_str!("../../../services/agent/session.rs");
    assert!(
        src.contains("struct AIAssistantSession"),
        "AIAssistantSession must live in session.rs (T-SessionMigrate / L2-A)"
    );
    for field in [
        "current_session_id",
        "current_binding",
        "loaded_mcp_server",
        "current_generation",
        "chat_cancelled",
        "execute_cancelled",
    ] {
        assert!(
            src.contains(field),
            "AIAssistantSession must hold live context field `{field}`"
        );
    }
}

#[test]
fn t4_loop_runtime_no_longer_owns_binding_mcp_live_session_fields() {
    let src = include_str!("../../../services/agent/loop.rs");
    let body = runtime_struct_body(src);
    for forbidden in [
        "current_session_id",
        "current_binding",
        "loaded_mcp_server",
        "current_generation",
        "generation_seq",
        "bound_master_task_id",
        "bound_title",
        "chat_cancelled",
        "execute_cancelled",
    ] {
        assert!(
            !body.contains(forbidden),
            "loop::Runtime must not authoritatively own `{forbidden}` after SessionMigrate"
        );
    }
    // Orchestration leftovers may remain on Runtime.
    assert!(
        body.contains("busy") && body.contains("executing"),
        "Runtime keeps single-flight orchestration (busy/executing)"
    );
}

#[test]
fn t4_ai_assistant_session_is_sole_live_context_owner() {
    with_sandbox(|| {
        assert!(
            session::live_context_owner().current_session_id().is_none(),
            "fresh live owner has no session_id"
        );
        assert!(
            session::live_context_owner().current_binding().is_none(),
            "fresh live owner is unbound"
        );

        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY }))
            .expect("key-only Set");
        let live = session::live_context_owner();
        assert!(
            live.current_binding().is_some(),
            "binding authority is AIAssistantSession"
        );
        assert!(
            live.loaded_mcp_server().is_some(),
            "MCP capability authority is AIAssistantSession"
        );
        assert!(
            live.current_generation().is_some(),
            "generation lives on AIAssistantSession"
        );

        let ensured = r#loop::ensure_chat_session_core().expect("ensure");
        let sid = ensured["session_id"].as_str().expect("session_id");
        assert!(!sid.is_empty());
        assert_eq!(
            session::live_context_owner().current_session_id().as_deref(),
            Some(sid),
            "session_id allocation / live id owned by AIAssistantSession"
        );

        // Public loop faces still work; they must read the same sole owner.
        assert_eq!(r#loop::binding_state(), "bound");
        assert!(r#loop::loaded_mcp_server().is_some());
        assert_eq!(
            r#loop::loaded_mcp_server().as_ref(),
            live.loaded_mcp_server().as_ref()
        );
    });
}

#[test]
fn t4_session_cache_remains_only_persistence_backend_no_parallel_store() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY }))
            .expect("Set");
        let ensured = r#loop::ensure_chat_session_core().expect("ensure");
        let sid = ensured["session_id"].as_str().unwrap().to_string();

        // Persist a turn through the existing Session cache API.
        let mut cached = session::load_session(&sid).expect("cache load");
        cached.turns.push(session::Turn {
            role: "user".into(),
            content: Some("hello from cache".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&cached).expect("cache save");

        let live = session::live_context_owner();
        assert_eq!(live.current_session_id().as_deref(), Some(sid.as_str()));
        // Live owner holds a reference/id — not a parallel turns store.
        let snap = live.execution_context_snapshot();
        assert_eq!(snap.session_id.as_deref(), Some(sid.as_str()));
        assert!(
            !AIAssistantSession::HOLDS_PARALLEL_TURN_STORAGE,
            "must not establish a parallel session store"
        );

        let reloaded = session::load_session(&sid).expect("reload");
        assert_eq!(reloaded.turns.len(), 1);
        assert_eq!(
            reloaded.turns[0].content.as_deref(),
            Some("hello from cache")
        );
        // Engine-opaque cache schema: no engine selection on Session record.
        let v = serde_json::to_value(&reloaded).expect("serialize");
        assert!(
            !session::value_exposes_engine_selection(&v),
            "Session cache stays engine-opaque"
        );
    });
}

#[test]
fn t4_per_binding_mcp_isolation_preserved() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY }))
            .expect("first Set");
        let first_mcp = r#loop::loaded_mcp_server().expect("mcp");
        let first_gen = session::live_context_owner()
            .current_generation()
            .expect("gen");
        let s1 = r#loop::ensure_chat_session_core().expect("s1")["session_id"]
            .as_str()
            .unwrap()
            .to_string();

        // Replace Set: new generation + MCP reload + session cut (clear-first).
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY }))
            .expect("replace Set");
        let live = session::live_context_owner();
        assert!(live.current_session_id().is_none(), "replace clears live id");
        assert_ne!(live.current_generation(), Some(first_gen));
        assert_eq!(
            live.loaded_mcp_server()
                .as_ref()
                .map(|c| &c.capability_description),
            Some(&first_mcp.capability_description)
        );

        let s2 = r#loop::ensure_chat_session_core().expect("s2")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(s1, s2, "new live session after cut");
        // Old cache file still exists (isolation: cut clears live id, not disk wipe).
        assert!(session::load_session(&s1).is_ok());
        assert!(session::load_session(&s2).is_ok());

        r#loop::reset_binding().expect("reset");
        let live = session::live_context_owner();
        assert!(live.current_binding().is_none());
        assert!(live.loaded_mcp_server().is_none());
        assert!(live.current_session_id().is_none());
        assert!(live.current_generation().is_none());
    });
}

#[test]
fn t4_engine_model_config_not_business_session_state() {
    let session_src = include_str!("../../../services/agent/session.rs");
    // AIAssistantSession / Session must not pin engine/model as business session state.
    let ai_start = session_src
        .find("struct AIAssistantSession")
        .expect("AIAssistantSession");
    let ai_body = &session_src[ai_start..];
    let ai_end = ai_body.find("\n}").expect("close");
    let ai_struct = &ai_body[..ai_end];
    for forbidden in ["engine", "model", "assistant_engine", "LlmEngine"] {
        assert!(
            !ai_struct.contains(forbidden),
            "AIAssistantSession must not hold engine/model field `{forbidden}`"
        );
    }
    assert!(
        session::SESSION_LIFECYCLE_ENGINE_OPAQUE,
        "session lifecycle remains engine-opaque"
    );
}

#[test]
fn t4_no_dual_authority_between_runtime_and_ai_assistant_session() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY }))
            .expect("Set");
        let _ = r#loop::ensure_chat_session_core().expect("ensure");

        // Sole owner snapshot is the authority; loop faces are facades over it.
        let snap = session::live_context_owner().execution_context_snapshot();
        assert!(snap.binding.is_some());
        assert!(snap.loaded_mcp_server.is_some());
        assert!(snap.session_id.is_some());

        assert_eq!(
            r#loop::current_binding_clone().as_ref(),
            snap.binding.as_ref()
        );
        assert_eq!(
            r#loop::loaded_mcp_server().as_ref(),
            snap.loaded_mcp_server.as_ref()
        );

        // Mutating via Binding Contract Reset clears the sole owner — no Runtime shadow copy.
        r#loop::reset_binding().expect("reset");
        let snap2 = session::live_context_owner().execution_context_snapshot();
        assert!(snap2.binding.is_none());
        assert!(snap2.loaded_mcp_server.is_none());
        assert!(snap2.session_id.is_none());
        assert!(r#loop::current_binding_clone().is_none());
        assert!(r#loop::loaded_mcp_server().is_none());
    });
}

#[test]
fn t4_cancel_state_owned_by_ai_assistant_session_not_runtime() {
    with_sandbox(|| {
        let binding = Binding {
            tools: json!([]),
            prompt: json!("prompt"),
            callbacks: json!({}),
        };
        r#loop::set_binding(binding).expect("Set");
        r#loop::set_busy_for_tests(true);
        r#loop::reset_binding().expect("reset cancels in-flight chat");
        assert!(
            session::live_context_owner().chat_cancelled(),
            "cancel flag authority is AIAssistantSession"
        );
        // Runtime orchestration busy may still be set by tests; cancel flag is not on Runtime.
        let loop_src = include_str!("../../../services/agent/loop.rs");
        let body = runtime_struct_body(loop_src);
        assert!(!body.contains("chat_cancelled"));
        assert!(!body.contains("execute_cancelled"));
    });
}

#[test]
fn t8_binding_business_id_is_derived_from_key_only_binding_not_session_id() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY }))
            .expect("key-only Set");
        let binding = session::live_context_owner()
            .current_binding()
            .expect("current Binding");

        assert_eq!(
            session::binding_business_id(&binding).as_deref(),
            Some(SEEDED_BUSINESS_KEY)
        );
        assert!(
            session::binding_business_id(&binding)
                .as_deref()
                .is_some_and(|business_id| business_id != "session_id"),
            "business routing must never fall back to session_id"
        );
    });
}

#[test]
fn t8_old_app_keys_notes_and_todo_task_fail_host_binding() {
    with_sandbox(|| {
        for key in ["notes", "todo_task"] {
            let err = r#loop::try_set_binding_json(&json!({ "key": key }))
                .expect_err("old App key must fail");
            assert_eq!(err.as_code(), "unknown_key", "key={key}");
            assert!(session::live_context_owner().current_binding().is_none());
            assert!(session::live_context_owner().loaded_mcp_server().is_none());
        }
    });
}

#[test]
fn t8_empty_key_is_invalid_and_does_not_bind_workbench() {
    with_sandbox(|| {
        let err = r#loop::try_set_binding_json(&json!({ "key": "" }))
            .expect_err("empty key must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert!(session::live_context_owner().current_binding().is_none());
        assert_ne!(
            session::live_context_owner().current_business_id().as_deref(),
            Some(SEEDED_BUSINESS_KEY)
        );
    });
}
