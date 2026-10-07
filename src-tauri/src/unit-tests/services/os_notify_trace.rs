use super::*;

#[test]
fn parse_accepts_minted_shape_and_rejects_injection() {
    let minted = new_trace_id();
    assert!(minted.starts_with("trace_"));
    assert_eq!(parse_trace_id(&minted), Some(minted.as_str()));
    assert_eq!(parse_trace_id("trace_12345678"), Some("trace_12345678"));
    assert!(parse_trace_id("short").is_none());
    assert!(parse_trace_id("trace_bad\nid").is_none());
    assert!(parse_trace_id("trace id space").is_none());
}

#[test]
fn trace_from_scheme_reads_query() {
    assert_eq!(
        trace_from_scheme("spark://notes/open?id=a&path=p.md&trace=trace_12345678"),
        Some("trace_12345678")
    );
    assert_eq!(
        trace_from_scheme("spark://read-later/list?trace=trace_12345678"),
        Some("trace_12345678")
    );
    assert!(trace_from_scheme("spark://read-later/list").is_none());
}

#[test]
fn scheme_query_fields_reads_id_path_trace() {
    let fields = scheme_query_fields(
        "spark://notes/open?id=abc&path=inbox%2Fx.md&trace=trace_12345678",
    );
    assert_eq!(fields.get("id").and_then(|v| v.as_str()), Some("abc"));
    assert_eq!(
        fields.get("path").and_then(|v| v.as_str()),
        Some("inbox/x.md")
    );
    assert_eq!(
        fields.get("trace").and_then(|v| v.as_str()),
        Some("trace_12345678")
    );
    assert!(fields
        .get("scheme")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .starts_with("spark://notes/open"));
}

#[test]
fn production_module_declares_tests_without_test_bodies() {
    let prod = include_str!("../../services/os_notify_trace.rs");
    assert!(prod.contains("#[path = \"../unit-tests/services/os_notify_trace.rs\"]"));
    assert!(!prod.contains("#[test]"));
    assert!(prod.contains(NODE_MESSAGE_CENTER_RECEIVED));
    assert!(prod.contains(NODE_NOTIFY_SEND));
    assert!(prod.contains(NODE_CLICK_NATIVE));
    assert!(prod.contains(NODE_SCHEME_OPEN));
    assert!(prod.contains("app_log::log"));
    assert!(prod.contains("login_hop::try_log_scheme_open"));
    assert!(!prod.contains("assistant-diagnostic"));
    assert!(!prod.contains("os-notify-trace.jsonl"));
}
