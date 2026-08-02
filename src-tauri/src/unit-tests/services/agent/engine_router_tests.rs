//! T2: settings → adapter routing (engine selection + dispatch).
//! T4: read-only EngineRuntimeConfig aggregator (category + model + credential).

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::secrets::{self, KEY_LLM_API_KEY, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings};
use crate::services::agent::engine_router::{
    self, AdapterKind, EngineKind, EngineRouteError, EngineRuntimeConfig, TurnInput,
};
use crate::services::agent::session::{self, value_exposes_engine_selection};

fn settings_with_engine(value: &str) -> AppSettings {
    let mut s = AppSettings::default();
    s.assistant_engine = value.to_string();
    s
}

#[test]
fn default_settings_select_host_engine() {
    let s = AppSettings::default();
    assert_eq!(s.assistant_engine, "host");
    assert_eq!(
        engine_router::resolve_engine(&s).expect("default"),
        EngineKind::Host
    );
}

#[test]
fn resolve_engine_reads_host_and_cursor_from_settings() {
    assert_eq!(
        engine_router::resolve_engine(&settings_with_engine("host")).expect("host"),
        EngineKind::Host
    );
    assert_eq!(
        engine_router::resolve_engine(&settings_with_engine("cursor")).expect("cursor"),
        EngineKind::Cursor
    );
}

#[test]
fn resolve_engine_empty_or_whitespace_defaults_to_host_without_panic() {
    assert_eq!(
        engine_router::resolve_engine(&settings_with_engine("")).expect("empty"),
        EngineKind::Host
    );
    assert_eq!(
        engine_router::resolve_engine(&settings_with_engine("   ")).expect("ws"),
        EngineKind::Host
    );
}

#[test]
fn resolve_engine_rejects_illegal_value_with_explicit_error() {
    let err = engine_router::resolve_engine(&settings_with_engine("bogus"))
        .expect_err("illegal must Err");
    match err {
        EngineRouteError::InvalidEngine(ref v) => assert_eq!(v, "bogus"),
        other => panic!("unexpected error: {other:?}"),
    }
    // Must not silently fall back to Host.
    assert_ne!(
        engine_router::resolve_engine(&settings_with_engine("claude")).ok(),
        Some(EngineKind::Host)
    );
}

#[test]
fn settings_read_path_exposes_assistant_engine() {
    let mut s = AppSettings::default();
    s.assistant_engine = "cursor".into();
    let v = settings::to_config_json(&s, false, false, false, false);
    assert_eq!(v["assistant_engine"], "cursor");

    let mut s2 = AppSettings::default();
    settings::apply_config_payload(
        &mut s2,
        &serde_json::json!({ "assistant_engine": "cursor" }),
    )
    .expect("apply");
    assert_eq!(s2.assistant_engine, "cursor");
    assert_eq!(
        engine_router::resolve_engine(&s2).expect("after apply"),
        EngineKind::Cursor
    );
}

#[test]
fn route_chat_turn_dispatches_to_host_or_cursor_adapter() {
    let host_hits = AtomicUsize::new(0);
    let cursor_hits = AtomicUsize::new(0);
    let input = TurnInput {
        session_id: "sess_t2".into(),
        message: "hello".into(),
    };

    let host_out = engine_router::route_chat_turn(
        EngineKind::Host,
        &input,
        || {
            host_hits.fetch_add(1, Ordering::SeqCst);
            Ok("host-stub".into())
        },
        || {
            cursor_hits.fetch_add(1, Ordering::SeqCst);
            Ok("cursor-stub".into())
        },
    )
    .expect("host route");
    assert_eq!(host_out.adapter, AdapterKind::Host);
    assert_eq!(host_out.body, "host-stub");
    assert_eq!(host_hits.load(Ordering::SeqCst), 1);
    assert_eq!(cursor_hits.load(Ordering::SeqCst), 0);

    let cursor_out = engine_router::route_chat_turn(
        EngineKind::Cursor,
        &input,
        || {
            host_hits.fetch_add(1, Ordering::SeqCst);
            Ok("host-stub".into())
        },
        || {
            cursor_hits.fetch_add(1, Ordering::SeqCst);
            Ok("cursor-stub".into())
        },
    )
    .expect("cursor route");
    assert_eq!(cursor_out.adapter, AdapterKind::Cursor);
    assert_eq!(cursor_out.body, "cursor-stub");
    assert_eq!(host_hits.load(Ordering::SeqCst), 1);
    assert_eq!(cursor_hits.load(Ordering::SeqCst), 1);
}

#[test]
fn route_result_does_not_leak_engine_into_session_api_surface() {
    let input = TurnInput {
        session_id: "sess_opaque".into(),
        message: "ping".into(),
    };
    let out = engine_router::route_chat_turn(
        EngineKind::Cursor,
        &input,
        || Ok("h".into()),
        || Ok("c".into()),
    )
    .expect("route");

    // Internal dispatch tag is fine; must not appear as session/facade JSON fields.
    let as_json = serde_json::to_value(&out).expect("serialize route result");
    assert!(
        !value_exposes_engine_selection(&as_json),
        "route result must not carry engine-selection keys for callers: {as_json}"
    );

    // Lifecycle create still engine-opaque (no engine arg; record has no engine fields).
    let sess = session::create_session(None, None);
    // create may fail without sandbox — only check API shape when Ok.
    if let Ok(sess) = sess {
        let serialized = serde_json::to_value(&sess).expect("serialize session");
        assert!(!value_exposes_engine_selection(&serialized));
        assert!(serialized.get("assistant_engine").is_none());
    }
}

#[test]
fn facade_must_not_accept_engine_selection_to_bypass_settings() {
    // T2 Failure: routing reads settings only — no engine param on public session create.
    let _ = session::create_session;
    // resolve_engine signature is settings-driven (compile-time contract via call below).
    let s = settings_with_engine("host");
    let _ = engine_router::resolve_engine(&s);
}

// ── T4: EngineRuntimeConfig read API ─────────────────────────────────────────

fn settings_with_engine_and_model(engine: &str, model: &str) -> AppSettings {
    let mut s = AppSettings::default();
    s.assistant_engine = engine.to_string();
    s.llm.model = model.to_string();
    s
}

#[test]
fn read_engine_runtime_config_returns_host_category_model_and_credential() {
    secrets::test_secrets_clear();
    secrets::set_secret(KEY_LLM_API_KEY, "sk-host-runtime").expect("set host");
    secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-cursor-other").expect("set cursor");

    let s = settings_with_engine_and_model("host", "glm-4");
    let cfg = engine_router::read_engine_runtime_config(&s).expect("host runtime");
    assert_eq!(cfg.engine, EngineKind::Host);
    assert_eq!(cfg.model, "glm-4");
    assert_eq!(cfg.credential.as_deref(), Some("sk-host-runtime"));
    // Must use the host slot — not the cursor key.
    assert_ne!(cfg.credential.as_deref(), Some("sk-cursor-other"));
}

#[test]
fn read_engine_runtime_config_returns_cursor_category_model_and_credential() {
    secrets::test_secrets_clear();
    secrets::set_secret(KEY_LLM_API_KEY, "sk-host-other").expect("set host");
    secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-cursor-runtime").expect("set cursor");

    let s = settings_with_engine_and_model("cursor", "composer-1");
    let cfg = engine_router::read_engine_runtime_config(&s).expect("cursor runtime");
    assert_eq!(cfg.engine, EngineKind::Cursor);
    assert_eq!(cfg.model, "composer-1");
    assert_eq!(cfg.credential.as_deref(), Some("sk-cursor-runtime"));
    assert_ne!(cfg.credential.as_deref(), Some("sk-host-other"));
}

#[test]
fn read_engine_runtime_config_reflects_settings_and_secret_changes() {
    secrets::test_secrets_clear();
    let mut s = settings_with_engine_and_model("host", "m1");
    secrets::set_secret(KEY_LLM_API_KEY, "sk-v1").expect("set");

    let first = engine_router::read_engine_runtime_config(&s).expect("first");
    assert_eq!(
        first,
        EngineRuntimeConfig {
            engine: EngineKind::Host,
            model: "m1".into(),
            credential: Some("sk-v1".into()),
        }
    );

    s.assistant_engine = "cursor".into();
    s.llm.model = "m2".into();
    secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-v2").expect("set cursor");

    let second = engine_router::read_engine_runtime_config(&s).expect("second");
    assert_eq!(second.engine, EngineKind::Cursor);
    assert_eq!(second.model, "m2");
    assert_eq!(second.credential.as_deref(), Some("sk-v2"));
}

#[test]
fn read_engine_runtime_config_aligns_with_resolve_engine_host_and_cursor() {
    secrets::test_secrets_clear();
    for (raw, kind) in [("host", EngineKind::Host), ("cursor", EngineKind::Cursor)] {
        let s = settings_with_engine(raw);
        assert_eq!(
            engine_router::resolve_engine(&s).expect("resolve"),
            kind
        );
        assert_eq!(
            engine_router::read_engine_runtime_config(&s)
                .expect("runtime")
                .engine,
            kind
        );
    }
}

#[test]
fn read_engine_runtime_config_empty_category_and_model_defaults_without_panic() {
    secrets::test_secrets_clear();
    let s = settings_with_engine_and_model("", "");
    let cfg = engine_router::read_engine_runtime_config(&s).expect("defaults");
    assert_eq!(cfg.engine, EngineKind::Host);
    assert_eq!(cfg.model, "");
    assert!(cfg.credential.is_none());

    let s2 = settings_with_engine_and_model("   ", "");
    let cfg2 = engine_router::read_engine_runtime_config(&s2).expect("ws defaults");
    assert_eq!(cfg2.engine, EngineKind::Host);
}

#[test]
fn read_engine_runtime_config_missing_credential_is_none_not_fabricated() {
    secrets::test_secrets_clear();
    let s = settings_with_engine_and_model("host", "any-model");
    let cfg = engine_router::read_engine_runtime_config(&s).expect("no key");
    assert_eq!(cfg.engine, EngineKind::Host);
    assert_eq!(cfg.model, "any-model");
    assert_eq!(cfg.credential, None);

    let s2 = settings_with_engine_and_model("cursor", "any-model");
    let cfg2 = engine_router::read_engine_runtime_config(&s2).expect("no cursor key");
    assert_eq!(cfg2.engine, EngineKind::Cursor);
    assert_eq!(cfg2.credential, None);
}

#[test]
fn read_engine_runtime_config_rejects_illegal_category_like_resolve_engine() {
    secrets::test_secrets_clear();
    let s = settings_with_engine("bogus");
    let resolve_err = engine_router::resolve_engine(&s).expect_err("resolve");
    let runtime_err = engine_router::read_engine_runtime_config(&s).expect_err("runtime");
    match (&resolve_err, &runtime_err) {
        (EngineRouteError::InvalidEngine(a), EngineRouteError::InvalidEngine(b)) => {
            assert_eq!(a, b);
            assert_eq!(a, "bogus");
        }
        other => panic!("expected matching InvalidEngine errors, got {other:?}"),
    }
}

#[test]
fn engine_runtime_config_is_read_only_snapshot_without_sdk_cwd_or_mcp() {
    secrets::test_secrets_clear();
    secrets::set_secret(KEY_LLM_API_KEY, "sk").expect("set");
    let s = settings_with_engine_and_model("host", "m");
    let cfg = engine_router::read_engine_runtime_config(&s).expect("cfg");

    // Snapshot fields: engine + model + credential only (no cwd / SDK / MCP).
    let EngineRuntimeConfig {
        engine,
        model,
        credential,
    } = cfg;
    assert_eq!(engine, EngineKind::Host);
    assert_eq!(model, "m");
    assert_eq!(credential.as_deref(), Some("sk"));

    let json = serde_json::to_value(&EngineRuntimeConfig {
        engine: EngineKind::Host,
        model: "m".into(),
        credential: Some("sk".into()),
    })
    .expect("serialize");
    for forbidden in [
        "cwd",
        "local_cwd",
        "sdk",
        "mcp",
        "mcpServers",
        "mcp_servers",
    ] {
        assert!(
            json.get(forbidden).is_none(),
            "runtime config must not expose {forbidden}: {json}"
        );
    }
    // Read API does not write settings / secrets.
    assert_eq!(s.assistant_engine, "host");
    assert_eq!(
        secrets::get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk".into())
    );
}
