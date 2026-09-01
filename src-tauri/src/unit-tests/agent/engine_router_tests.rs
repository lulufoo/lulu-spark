//! Host-only engine routing and unconfigured legacy settings.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings::{self, AppSettings};
use crate::agent::engine_router::{
    self, AdapterKind, EngineKind, EngineRouteError, EngineRuntimeConfig, TurnInput,
};
use crate::agent::session::{self, value_exposes_engine_selection};
use crate::test_support::TestSandbox;

fn settings_with_engine(value: &str) -> AppSettings {
    let mut settings = AppSettings::default();
    settings.assistant_engine = value.to_string();
    settings
}

fn host_settings_with_model(model: &str) -> AppSettings {
    let mut settings = settings_with_engine("host");
    settings::upsert_llm_entry(
        &mut settings.llm,
        "host",
        &settings::LlmSettings {
            model: model.to_string(),
            ..Default::default()
        },
    )
    .expect("host entry");
    settings
}

#[test]
fn resolve_engine_accepts_only_host_and_treats_legacy_values_as_unconfigured() {
    assert_eq!(
        engine_router::resolve_engine(&settings_with_engine("host")).expect("host"),
        EngineKind::Host
    );

    for raw in ["", "   ", "cursor", "bogus"] {
        assert!(
            matches!(
                engine_router::resolve_engine(&settings_with_engine(raw)),
                Err(EngineRouteError::Unconfigured)
            ),
            "{raw:?} must not route to an Agent"
        );
    }
}

#[test]
fn read_engine_runtime_config_reads_only_the_glm_host_entry() {
    secrets::test_secrets_clear();
    secrets::set_secret(KEY_LLM_API_KEY, "sk-host").expect("host key");

    let settings = host_settings_with_model("glm-4");
    let config = engine_router::read_engine_runtime_config(&settings).expect("runtime config");

    assert_eq!(
        config,
        EngineRuntimeConfig {
            engine: EngineKind::Host,
            model: "glm-4".into(),
            credential: Some("sk-host".into()),
        }
    );
}

#[test]
fn read_engine_runtime_config_rejects_empty_and_cursor_without_fallback() {
    secrets::test_secrets_clear();
    for raw in ["", "cursor"] {
        let settings = settings_with_engine(raw);
        assert!(
            matches!(
                engine_router::read_engine_runtime_config(&settings),
                Err(EngineRouteError::Unconfigured)
            ),
            "{raw:?} must remain unconfigured"
        );
    }
}

#[test]
fn route_chat_turn_has_one_host_adapter_path() {
    let hits = AtomicUsize::new(0);
    let input = TurnInput {
        session_id: "session".into(),
        message: "hello".into(),
    };

    let outcome = engine_router::route_chat_turn(EngineKind::Host, &input, || {
        hits.fetch_add(1, Ordering::SeqCst);
        Ok("host".into())
    })
    .expect("host route");

    assert_eq!(outcome.adapter, AdapterKind::Host);
    assert_eq!(outcome.body, "host");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn dispatch_from_settings_does_not_invoke_host_for_legacy_cursor() {
    let hits = AtomicUsize::new(0);
    let input = TurnInput {
        session_id: "session".into(),
        message: "hello".into(),
    };

    let result = engine_router::dispatch_from_settings(
        &settings_with_engine("cursor"),
        &input,
        || {
            hits.fetch_add(1, Ordering::SeqCst);
            Ok("must not run".into())
        },
    );

    assert!(matches!(result, Err(EngineRouteError::Unconfigured)));
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

#[test]
fn route_result_does_not_expose_engine_selection_to_session_callers() {
    let input = TurnInput {
        session_id: "session".into(),
        message: "ping".into(),
    };
    let outcome = engine_router::route_chat_turn(EngineKind::Host, &input, || Ok("ok".into()))
        .expect("route");

    let json = serde_json::to_value(&outcome).expect("serialize");
    assert!(!value_exposes_engine_selection(&json), "route leaked engine: {json}");

    let _sandbox = TestSandbox::new();
    let session = session::create_session().expect("isolated session");
    let json = serde_json::to_value(&session).expect("serialize session");
    assert!(!value_exposes_engine_selection(&json));
    assert!(json.get("assistant_engine").is_none());
}
