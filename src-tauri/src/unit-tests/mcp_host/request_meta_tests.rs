use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::{header, Method};
use serde_json::json;

use super::{http_log_params, peek_request, Caller, RequestInfo, PEEK_LIMIT};
use crate::agent::diagnostics::{TraceId, TurnId};

const SESSION: &str = "spark_chat_5141a723ad70f2cadd4e5885a5421b06";

fn run<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime")
        .block_on(future)
}

fn post(body: &str, declare_length: bool) -> Request {
    let mut builder = Request::builder().method(Method::POST).uri("/");
    if declare_length {
        builder = builder.header(header::CONTENT_LENGTH, body.len());
    }
    builder.body(Body::from(body.to_string())).expect("request")
}

/// Peek, then read the forwarded body back to prove it is untouched.
fn peek(request: Request) -> (RequestInfo, String) {
    run(async {
        let (request, info) = peek_request(request).await.expect("peek");
        let forwarded = to_bytes(request.into_body(), usize::MAX).await.expect("body");
        (info, String::from_utf8(forwarded.to_vec()).expect("utf8"))
    })
}

#[test]
fn json_rpc_method_and_meta_ids_are_read_and_the_body_is_forwarded_intact() {
    let (trace_id, turn_id) = (TraceId::new(), TurnId::new());
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list",
        "params": { "_meta": {
            "spark/trace_id": trace_id.as_str(),
            "spark/turn_id": turn_id.as_str(),
            "spark/session_id": SESSION,
        } },
    })
    .to_string();

    let (info, forwarded) = peek(post(&body, true));

    assert_eq!(info.method, "tools/list");
    assert_eq!(info.meta.trace_id, Some(trace_id));
    assert_eq!(info.meta.turn_id, Some(turn_id));
    assert_eq!(info.meta.session_id.as_deref(), Some(SESSION));
    assert_eq!(forwarded, body);
}

#[test]
fn request_without_meta_keeps_its_method_and_has_no_ids() {
    let body = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();

    let (info, forwarded) = peek(post(&body, true));

    assert_eq!(info.method, "notifications/initialized");
    assert_eq!(info.meta.trace_id, None);
    assert_eq!(info.meta.turn_id, None);
    assert_eq!(info.meta.session_id, None);
    assert_eq!(forwarded, body);
}

#[test]
fn invalid_meta_ids_are_dropped() {
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "_meta": {
            "spark/trace_id": "bad id\nwith newline",
            "spark/turn_id": "bad id\nwith newline",
            "spark/session_id": "../escape",
        } },
    })
    .to_string();

    let (info, _) = peek(post(&body, true));

    assert_eq!(info.method, "initialize");
    assert_eq!(info.meta.trace_id, None);
    assert_eq!(info.meta.turn_id, None);
    assert_eq!(info.meta.session_id, None);
}

#[test]
fn bodies_that_are_not_a_single_request_fall_back_to_the_verb() {
    for body in ["not json", "[{\"method\":\"tools/list\"}]", "{\"result\":{}}"] {
        let (info, forwarded) = peek(post(body, true));
        assert_eq!(info.method, "POST", "body {body:?}");
        assert_eq!(forwarded, body, "body {body:?} must be forwarded intact");
    }
}

#[test]
fn bodies_of_unknown_or_large_length_are_not_read() {
    let body = json!({ "jsonrpc": "2.0", "method": "tools/list" }).to_string();
    let (undeclared, forwarded) = peek(post(&body, false));
    assert_eq!(undeclared.method, "POST");
    assert_eq!(forwarded, body);

    let large = "x".repeat(PEEK_LIMIT + 1);
    let (oversized, forwarded) = peek(post(&large, true));
    assert_eq!(oversized.method, "POST");
    assert_eq!(forwarded.len(), large.len());
}

#[test]
fn non_post_requests_report_their_verb() {
    for method in [Method::GET, Method::DELETE] {
        let request = Request::builder()
            .method(method.clone())
            .uri("/")
            .body(Body::empty())
            .expect("request");
        let (info, _) = peek(request);
        assert_eq!(info.method, method.to_string());
        assert_eq!(info.meta.session_id, None);
    }
}

#[test]
fn http_log_params_carry_component_method_fields_session_and_turn() {
    let turn_id = TurnId::new();
    let info = RequestInfo {
        method: "tools/list".into(),
        meta: crate::agent::mcp::TraceMeta {
            trace_id: None,
            turn_id: Some(turn_id.clone()),
            session_id: Some(SESSION.into()),
        },
        ..Default::default()
    };

    let caller = Caller::from_headers("spark", &axum::http::HeaderMap::new());
    let params = http_log_params(&info, &caller, json!({ "outcome": "ok", "http_status": 200 }))
        .expect("params");

    assert_eq!(params["component"], "mcp_host.http");
    assert_eq!(params["method"], "tools/list");
    assert_eq!(params["outcome"], "ok");
    assert_eq!(params["http_status"], 200);
    assert_eq!(params["session_id"], SESSION);
    assert_eq!(params["turn_id"], turn_id.as_str());

    assert_eq!(params["slot"], "spark");

    let without = http_log_params(&RequestInfo::default(), &caller, json!({})).expect("params");
    assert!(without.get("session_id").is_none());
    assert!(without.get("turn_id").is_none());
}

fn initialize_body(client_info: serde_json::Value) -> String {
    json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": client_info },
    })
    .to_string()
}

#[test]
fn initialize_reports_the_client_name_and_version() {
    let body = initialize_body(json!({ "name": "cursor-vscode", "version": "1.2.3" }));
    let (info, _) = peek(post(&body, true));
    assert_eq!(info.client_name.as_deref(), Some("cursor-vscode"));
    assert_eq!(info.client_version.as_deref(), Some("1.2.3"));

    let caller = Caller::from_headers("spark", &axum::http::HeaderMap::new());
    let params = http_log_params(&info, &caller, json!({})).expect("params");
    assert_eq!(params["client_name"], "cursor-vscode");
    assert_eq!(params["client_version"], "1.2.3");
}

#[test]
fn client_name_is_cleaned_and_cut_and_only_read_from_initialize() {
    let body = initialize_body(json!({ "name": format!("bad\nname{}", "x".repeat(200)), "version": "1\r.0" }));
    let (info, _) = peek(post(&body, true));
    let name = info.client_name.expect("name");
    assert!(!name.contains('\n') && name.starts_with("badname"));
    assert_eq!(name.chars().count(), 64);
    assert_eq!(info.client_version.as_deref(), Some("1.0"));

    let list = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list",
        "params": { "clientInfo": { "name": "spoof", "version": "9" } },
    })
    .to_string();
    let (info, _) = peek(post(&list, true));
    assert_eq!((info.client_name, info.client_version), (None, None));
}
