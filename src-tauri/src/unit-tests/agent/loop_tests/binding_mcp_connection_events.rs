//! The live MCP connection writes its handshake / list / reset events to the
//! chat session's own `session.log`.

use serde_json::{json, Value};

use super::support::*;
use crate::agent::diagnostics::{self, TurnId};
use crate::agent::mcp::{self, McpTurn, ResetReason};

// Shared fixture compiled into several test modules; each uses a subset.
#[allow(dead_code)]
#[path = "../mcp_wire_server.rs"]
mod wire_server;

use wire_server::WireServer;

const SESSION: &str = "spark_chat_0123456789abcdef0123456789abcdef";

fn turn() -> McpTurn {
    McpTurn::new(&TurnId::new(), SESSION)
}

/// Every `assistant.mcp` event of the session, in order.
fn mcp_events() -> Vec<Value> {
    let path = diagnostics::session_log_path(SESSION).expect("session log path");
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|event| event["component"] == "assistant.mcp")
        .collect()
}

/// The connection events: everything but the per-call request events.
fn connection_events() -> Vec<Value> {
    mcp_events()
        .into_iter()
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

        mcp::discover_tools(&config, &turn()).expect("first list");
        mcp::discover_tools(&config, &turn()).expect("second list");

        let events = connection_events();
        assert_eq!(
            names(&events),
            [
                "connect.started",
                "connect.completed",
                "list.attempt.completed",
                "list.attempt.completed"
            ]
        );
        for event in &events {
            assert_eq!(event["schema_version"], 2);
            assert_eq!(event["session_id"], SESSION);
            assert!(event["turn_id"].as_str().is_some_and(|id| id.starts_with("turn_")));
            let is_request = event["event"]
                .as_str()
                .is_some_and(|name| name.starts_with("connect.") || name.starts_with("list.attempt"));
            assert_eq!(event.get("trace_id").is_some(), is_request, "{event}");
        }
        assert_eq!(events[0]["trace_id"], events[1]["trace_id"], "one handshake request");
        assert_eq!(
            events[0]["trace_id"],
            server.metas("initialize")[0]["spark/trace_id"],
            "the client line carries the id the server received"
        );
        assert_eq!(events[1]["fields"]["outcome"], "ok");
        assert_eq!(events[2]["fields"]["attempt"], 1);
        assert_eq!(events[2]["fields"]["reused"], false);
        assert_eq!(events[2]["fields"]["tool_count"], 1);
        assert_eq!(events[3]["fields"]["attempt"], 1);
        assert_eq!(events[3]["fields"]["reused"], true);
        let sent = server.metas("tools/list");
        assert_eq!(events[2]["trace_id"], sent[0]["spark/trace_id"]);
        assert_eq!(events[3]["trace_id"], sent[1]["spark/trace_id"]);
        assert_ne!(events[2]["trace_id"], events[3]["trace_id"]);
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn failed_list_on_a_reused_connection_logs_reset_reconnect_and_retry() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        mcp::discover_tools(&config, &turn()).expect("list while healthy");

        server.fail_next("tools/list", 1);
        mcp::discover_tools(&config, &turn()).expect("retry succeeds");

        let events = connection_events();
        assert_eq!(
            names(&events)[2..],
            [
                "list.attempt.completed",
                "list.attempt.completed",
                "connection.reset",
                "connect.started",
                "connect.completed",
                "list.attempt.completed"
            ]
        );
        let (first, failed, reset, retried) = (&events[2], &events[3], &events[4], &events[7]);
        assert_eq!(first["fields"]["outcome"], "ok");
        assert_eq!(failed["fields"]["attempt"], 1);
        assert_eq!(failed["fields"]["reused"], true);
        assert_eq!(failed["fields"]["outcome"], "error");
        assert!(failed["fields"]["sdk_error_message"].is_string(), "{failed}");
        assert_eq!(reset["fields"]["reason"], "list_failed");
        assert_eq!(reset["trace_id"], failed["trace_id"], "the reset belongs to the failed request");
        assert_eq!(retried["fields"]["attempt"], 2);
        assert_eq!(retried["fields"]["reused"], false);
        assert_eq!(retried["fields"]["outcome"], "ok");
        let sent = server.metas("tools/list");
        assert_eq!(sent.len(), 3);
        for (event, meta) in [first, failed, retried].into_iter().zip(&sent) {
            assert_eq!(event["trace_id"], meta["spark/trace_id"]);
        }
        let traces = [&first["trace_id"], &failed["trace_id"], &retried["trace_id"]];
        assert!(traces[0] != traces[1] && traces[1] != traces[2] && traces[0] != traces[2]);
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn list_failing_on_both_attempts_logs_two_error_events() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        mcp::discover_tools(&config, &turn()).expect("list while healthy");

        server.fail_next("tools/list", 2);
        assert!(mcp::discover_tools(&config, &turn()).is_err());

        let attempts: Vec<_> = connection_events()
            .into_iter()
            .filter(|event| event["event"] == "list.attempt.completed")
            .skip(1)
            .collect();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0]["fields"]["attempt"], 1);
        assert_eq!(attempts[1]["fields"]["attempt"], 2);
        for attempt in &attempts {
            assert_eq!(attempt["fields"]["outcome"], "error");
            assert!(attempt["fields"].get("tool_count").is_none());
        }
        assert_ne!(attempts[0]["trace_id"], attempts[1]["trace_id"]);
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn failed_tool_call_logs_a_call_failed_reset() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        mcp::discover_tools(&config, &turn()).expect("list");

        server.fail_next("tools/call", 1);
        let failed = mcp::call_tool(&config, "ping", json!({}), &TurnId::new(), SESSION);
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
        mcp::discover_tools(&server.config(&[("x-ticket", "one")]), &turn()).expect("first");
        mcp::discover_tools(&server.config(&[("x-ticket", "two")]), &turn()).expect("second");

        let events = connection_events();
        assert_eq!(
            names(&events)[3..],
            ["connection.reset", "connect.started", "connect.completed", "list.attempt.completed"]
        );
        assert_eq!(events[3]["fields"]["reason"], "config_changed");
        assert!(events[3].get("trace_id").is_none(), "dropped before any request of this turn");
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn each_call_request_logs_the_trace_id_the_server_received() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        let turn_id = TurnId::new();

        for _ in 0..2 {
            mcp::call_tool(&config, "ping", json!({}), &turn_id, SESSION).expect("call");
        }

        let completed: Vec<Value> = mcp_events()
            .into_iter()
            .filter(|event| event["event"] == "call.request.completed")
            .collect();
        let sent = server.metas("tools/call");
        assert_eq!((completed.len(), sent.len()), (2, 2));
        for (event, meta) in completed.iter().zip(&sent) {
            assert_eq!(event["trace_id"], meta["spark/trace_id"]);
            assert_eq!(event["turn_id"], turn_id.as_str());
            assert_eq!(event["session_id"], SESSION);
        }
        assert_ne!(
            completed[0]["trace_id"], completed[1]["trace_id"],
            "two calls of one tool in one turn must be told apart"
        );
        let started: Vec<Value> = mcp_events()
            .into_iter()
            .filter(|event| event["event"] == "call.request.started")
            .collect();
        assert_eq!(started[0]["trace_id"], completed[0]["trace_id"]);
        assert_eq!(started[1]["trace_id"], completed[1]["trace_id"]);
        mcp::release_connection(ResetReason::BindingReset);
    });
}
