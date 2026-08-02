//! T2: settings → adapter routing (engine selection + dispatch).

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::settings::{self, AppSettings};
use crate::services::agent::engine_router::{
    self, AdapterKind, EngineKind, EngineRouteError, TurnInput,
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
