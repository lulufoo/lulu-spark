use super::*;
use crate::services::login_hop::pass_id_from_scheme;

fn pass_bag(id: &str) -> String {
    urlencoding::encode(&format!(r#"{{"id":"{id}"}}"#)).into_owned()
}

fn scheme_with_pass(host_path: &str, id: &str) -> String {
    format!("spark://{host_path}?pass={}", pass_bag(id))
}

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
fn business_from_scheme_host_maps_valid_pass_notes_and_read_later() {
    let id = "trace_12345678abcd";
    assert_eq!(
        business_from_scheme_host(&format!(
            "spark://notes/open?id=a&path=p.md&pass={}",
            pass_bag(id)
        )),
        "notes"
    );
    assert_eq!(
        business_from_scheme_host(&scheme_with_pass("notes/open", id)),
        "notes"
    );
    assert_eq!(
        business_from_scheme_host(&scheme_with_pass("read-later/list", id)),
        "read_later"
    );
}

#[test]
fn business_from_scheme_host_maps_valid_pass_auth_login_to_login() {
    let id = "trace_12345678abcd";
    assert_eq!(
        business_from_scheme_host(&scheme_with_pass("auth-login/callback", id)),
        "login"
    );
}

#[test]
fn business_from_scheme_host_unmapped_spark_host_is_app() {
    let id = "trace_12345678abcd";
    assert_eq!(
        business_from_scheme_host(&scheme_with_pass("settings/open", id)),
        "app"
    );
}

#[test]
fn leftover_missing_pass_or_trace_only_is_app_not_host_mapped() {
    for scheme in [
        "spark://notes/open?id=a&path=p.md",
        "spark://notes/open?id=a&path=p.md&trace=trace_12345678",
        "spark://read-later/list?trace=trace_12345678",
        "spark://read-later/list",
        "spark://auth-login/callback",
        "spark://auth-login/callback?trace=trace_12345678abcd",
        "spark://notes/open?trace=trace_12345678abcd",
        "",
    ] {
        assert_eq!(
            business_from_scheme_host(scheme),
            "app",
            "leftover must log app: {scheme:?}"
        );
        assert!(
            pass_id_from_scheme(scheme).is_none(),
            "leftover has no pass hop id: {scheme:?}"
        );
    }
}

#[test]
fn hop_id_comes_from_pass_bag_not_trace_query() {
    let id = "trace_12345678abcd";
    let bag = pass_bag(id);
    let notes = format!("spark://notes/open?id=note1&path=p.md&pass={bag}&trace=trace_99999999zzzz");
    assert_eq!(pass_id_from_scheme(&notes).as_deref(), Some(id));
    assert_eq!(business_from_scheme_host(&notes), "notes");
    let leftover = "spark://notes/open?trace=trace_12345678abcd";
    assert!(pass_id_from_scheme(leftover).is_none());
    assert_eq!(business_from_scheme_host(leftover), "app");
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
    assert!(prod.contains("fn business_from_scheme_host"));
    assert!(prod.contains("pass_id_from_scheme"));
    assert!(prod.contains("fn log_hop(business: &str, node: &str, hop_id: &str, outcome: &str)"));
    assert!(prod.contains("fn log_hop_extra("));
    assert!(prod.contains("business: &str"));
    assert!(prod.contains(NODE_MESSAGE_CENTER_RECEIVED));
    assert!(prod.contains(NODE_NOTIFY_SEND));
    assert!(prod.contains(NODE_CLICK_NATIVE));
    assert!(prod.contains(NODE_SCHEME_OPEN));
    assert!(prod.contains("app_log::log"));
    assert!(prod.contains("login_hop::try_log_scheme_open"));
    assert!(prod.contains("\"empty\""));
    assert!(!prod.contains("assistant-diagnostic"));
    assert!(!prod.contains("os-notify-trace.jsonl"));
    assert!(
        !prod.contains("\"os-notify\""),
        "notify hops must not hardcode business os-notify"
    );
    assert!(
        !prod.contains("fn trace_from_scheme") && !prod.contains("TRACE_QUERY"),
        "notify hops must not read hop id from trace="
    );
}
