use super::*;
use crate::test_support::TestSandbox;
use serde_json::{json, Value};
use std::fs;

#[test]
fn parse_accepts_minted_trace_and_rejects_injection() {
    let minted = new_trace_id();
    assert!(minted.starts_with("trace_"));
    assert_eq!(parse_trace_id(&minted), Some(minted.as_str()));
    assert!(parse_trace_id("short").is_none());
    assert!(parse_trace_id("trace_bad\nid").is_none());
}

#[test]
fn persists_one_process_file_with_base_and_body_fields() {
    let sandbox = TestSandbox::new();
    persist_for_test(
        "os-notify",
        "notify.send",
        Some("trace_12345678"),
        Some(json!({ "outcome": "ok", "business": "hijack" })),
        Side::Host,
        Level::Info,
    )
    .expect("persist");

    let path = log_path().expect("path");
    assert_eq!(
        path.parent().map(|p| p.to_path_buf()),
        Some(sandbox.cache_dir().join(DIR_NAME))
    );
    assert!(path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .starts_with("psess_"));
    assert!(!path.starts_with(sandbox.cache_dir().join("agent-exec")));

    let current: Value =
        serde_json::from_str(&fs::read_to_string(current_path().expect("current")).expect("read"))
            .expect("json");
    assert_eq!(current["process_session_id"], process_session_id());
    assert_eq!(current["path"], path.to_string_lossy().as_ref());

    let line = fs::read_to_string(&path)
        .expect("read log")
        .lines()
        .last()
        .expect("line")
        .to_string();
    let value: Value = serde_json::from_str(&line).expect("jsonl");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["process_session_id"], process_session_id());
    assert_eq!(value["side"], "host");
    assert_eq!(value["level"], "info");
    assert_eq!(value["business"], "os-notify");
    assert_eq!(value["trace_id"], "trace_12345678");
    assert_eq!(value["event"], "notify.send");
    assert!(
        line.contains(r#""business":"os-notify","trace_id":"trace_12345678","event":"notify.send""#),
        "trace_id follows business and precedes event: {line}"
    );
    assert_eq!(value["params"]["outcome"], "ok");
    assert!(value["params"].get("business").is_none());
    assert!(value["seq"].as_u64().unwrap_or(0) >= 1);
    assert!(value["timestamp"].as_str().unwrap_or("").contains("+08:00"));
}

#[test]
fn production_module_stays_off_agent_and_old_notify_file() {
    let prod = include_str!("../../services/app_log.rs");
    assert!(prod.contains("#[path = \"../unit-tests/services/app_log.rs\"]"));
    assert!(!prod.contains("#[test]"));
    assert!(!prod.contains("agent::diagnostics"));
    assert!(!prod.contains("DiagnosticEvent"));
    assert!(!prod.contains("os-notify-trace.jsonl"));
    assert!(!prod.contains("business.jsonl"));
}

#[test]
fn process_start_params_include_package_snapshot() {
    let sandbox = TestSandbox::new();
    let params = serde_json::to_value(crate::host::snapshot()).expect("json");
    persist_for_test(
        BUSINESS_APP,
        EVENT_PROCESS_START,
        None,
        Some(params.clone()),
        Side::Host,
        Level::Info,
    )
    .expect("persist");

    let line = fs::read_to_string(log_path().expect("path"))
        .expect("read log")
        .lines()
        .last()
        .expect("line")
        .to_string();
    let value: Value = serde_json::from_str(&line).expect("jsonl");
    assert_eq!(value["event"], EVENT_PROCESS_START);
    assert_eq!(value["params"]["is_debug"], params["is_debug"]);
    assert_eq!(value["params"]["version"], params["version"]);
    assert_eq!(value["params"]["product_name"], params["product_name"]);
    assert!(value.get("schema_version").is_some());
    assert!(value["params"].get("schema_version").is_none());
    assert!(value["params"].get("event").is_none());
    assert_eq!(
        sandbox.cache_dir().join(DIR_NAME),
        log_path().expect("path").parent().unwrap().to_path_buf()
    );
}

#[test]
fn process_start_init_wires_host_snapshot() {
    let prod = include_str!("../../services/app_log.rs");
    assert!(prod.contains("EVENT_PROCESS_START"));
    assert!(prod.contains("crate::host::snapshot"));
}
