//! The live MCP connection writes its handshake / list / reset events to the
//! chat session's own `session.log`.

use serde_json::{json, Value};

use super::support::*;
use crate::agent::diagnostics::{self, TraceId};
use crate::agent::mcp::{self, McpTrace, ResetReason};

// Shared fixture compiled into several test modules; each uses a subset.
#[allow(dead_code)]
#[path = "../mcp_wire_server.rs"]
mod wire_server;

use wire_server::WireServer;

const SESSION: &str = "spark_chat_0123456789abcdef0123456789abcdef";

fn trace() -> McpTrace {
    McpTrace::new(&TraceId::new(), SESSION)
}

/// `assistant.mcp` connection events of the session, in order.
fn connection_events() -> Vec<Value> {
    let path = diagnostics::session_log_path(SESSION).expect("session log path");
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|event| event["component"] == "assistant.mcp")
        .filter(|event| event["event"] != "call.request.started")
        .filter(|event| event["event"] != "call.request.completed")
        .collect()
}

fn names(events: &[Value]) -> Vec<&str> {
    events.iter().filter_map(|event| event["event"].as_str()).collect()
}

#[test]
fn first_list_handshakes_and_the_second_reuses_the_connection() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);

        mcp::discover_tools(&config, &trace()).expect("first list");
        mcp::discover_tools(&config, &trace()).expect("second list");

        let events = connection_events();
        assert_eq!(
            names(&events),
            ["connect.started", "connect.completed", "list.completed", "list.completed"]
        );
        assert_eq!(events[1]["fields"]["outcome"], "ok");
        assert_eq!(events[2]["fields"]["reused"], false);
        assert_eq!(events[2]["fields"]["tool_count"], 1);
        assert_eq!(events[3]["fields"]["reused"], true);
        assert_eq!(events[3]["fields"]["retried"], false);
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn failed_list_on_a_reused_connection_logs_reset_reconnect_and_retry() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        mcp::discover_tools(&config, &trace()).expect("list while healthy");

        server.fail_next("tools/list", 1);
        mcp::discover_tools(&config, &trace()).expect("retry succeeds");

        let events = connection_events();
        assert_eq!(
            names(&events)[3..],
            ["connection.reset", "connect.started", "connect.completed", "list.completed"]
        );
        assert_eq!(events[3]["fields"]["reason"], "list_failed");
        assert_eq!(events[6]["fields"]["reused"], true);
        assert_eq!(events[6]["fields"]["retried"], true);
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn failed_tool_call_logs_a_call_failed_reset() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        mcp::discover_tools(&config, &trace()).expect("list");

        server.fail_next("tools/call", 1);
        let failed = mcp::call_tool(&config, "ping", json!({}), &TraceId::new(), SESSION);
        assert!(failed.is_err());

        let events = connection_events();
        let reset = events.last().expect("reset event");
        assert_eq!(reset["event"], "connection.reset");
        assert_eq!(reset["fields"]["reason"], "call_failed");
    });
}

#[test]
fn changed_config_logs_a_config_changed_reset() {
    with_sandbox(|| {
        let server = WireServer::start();
        mcp::discover_tools(&server.config(&[("x-ticket", "one")]), &trace()).expect("first");
        mcp::discover_tools(&server.config(&[("x-ticket", "two")]), &trace()).expect("second");

        let events = connection_events();
        assert_eq!(
            names(&events)[3..],
            ["connection.reset", "connect.started", "connect.completed", "list.completed"]
        );
        assert_eq!(events[3]["fields"]["reason"], "config_changed");
        mcp::release_connection(ResetReason::BindingReset);
    });
}
