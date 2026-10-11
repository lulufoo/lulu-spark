use super::ConnectionSlot;
use crate::agent::mcp::McpTrace;
use crate::agent::mcp::McpError;
use rmcp::model::CallToolRequestParams;

#[path = "mcp_wire_server.rs"]
mod wire_server;

use wire_server::WireServer;

/// Events without a session go to the app log, so tests need no sandbox.
fn trace() -> McpTrace {
    McpTrace::detached()
}

fn params(tool: &str) -> CallToolRequestParams {
    CallToolRequestParams::new(tool.to_string())
}

#[test]
fn second_list_tools_reuses_the_connection_without_a_new_handshake() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);

    let first = slot.list_tools(&config, &trace()).expect("first list");
    let second = slot.list_tools(&config, &trace()).expect("second list");

    assert!(first.contains("ping") && second.contains("ping"));
    assert_eq!(
        server.calls(),
        ["initialize", "notifications/initialized", "tools/list", "tools/list"],
        "one handshake, two lists, and no DELETE while the connection is live"
    );
}

#[test]
fn changed_config_closes_the_old_connection_and_handshakes_again() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();

    slot.list_tools(&server.config(&[("x-ticket", "one")]), &trace())
        .expect("list with first config");
    slot.list_tools(&server.config(&[("x-ticket", "two")]), &trace())
        .expect("list with second config");

    assert_eq!(
        server.calls(),
        [
            "initialize",
            "notifications/initialized",
            "tools/list",
            "DELETE",
            "initialize",
            "notifications/initialized",
            "tools/list",
        ],
        "the first session is deleted before the second one handshakes"
    );
}

#[test]
fn failed_list_on_a_reused_connection_handshakes_again_and_retries_once() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);
    slot.list_tools(&config, &trace()).expect("list while healthy");

    server.fail_next("tools/list", 1);
    let recovered = slot.list_tools(&config, &trace()).expect("retry succeeds");

    assert!(recovered.contains("ping"));
    assert_eq!(
        server.calls(),
        [
            "initialize",
            "notifications/initialized",
            "tools/list",
            "tools/list",
            "DELETE",
            "initialize",
            "notifications/initialized",
            "tools/list",
        ],
        "the failed list drops the session, then one fresh handshake and one retry"
    );
}

#[test]
fn second_failure_is_reported_without_a_further_retry() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);
    slot.list_tools(&config, &trace()).expect("list while healthy");

    server.fail_next("tools/list", 2);
    let error = slot.list_tools(&config, &trace()).expect_err("both attempts fail");

    assert!(matches!(error, McpError::ListTools(_)), "{error}");
    let lists = server.calls().iter().filter(|call| *call == "tools/list").count();
    assert_eq!(lists, 3, "one healthy list, then exactly the failed attempt and one retry");
}

#[test]
fn failed_list_on_a_new_connection_is_not_retried() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);

    server.fail_next("tools/list", 1);
    let error = slot.list_tools(&config, &trace()).expect_err("fresh connection fails");

    assert!(matches!(error, McpError::ListTools(_)), "{error}");
    assert_eq!(
        server.calls(),
        ["initialize", "notifications/initialized", "tools/list", "DELETE"],
        "a connection that was just handshaken is dropped, not retried"
    );
    slot.list_tools(&config, &trace()).expect("the next call connects afresh");
}

#[test]
fn failed_list_drops_the_connection_so_the_next_call_connects_afresh() {
    let mut server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);
    slot.list_tools(&config, &trace()).expect("list while server is up");

    server.shut_down();

    let error = slot.list_tools(&config, &trace()).expect_err("server is gone");
    assert!(
        matches!(error, McpError::Connect(_)),
        "the retry after the reused connection failed has to handshake from scratch: {error}"
    );
    let next = slot.list_tools(&config, &trace()).expect_err("still no server");
    assert!(matches!(next, McpError::Connect(_)), "{next}");
}

#[test]
fn tool_calls_reuse_the_connection_opened_for_tools_list() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);
    slot.list_tools(&config, &trace()).expect("list");

    for _ in 0..2 {
        let result = slot.call_tool(&config, params("ping"), &trace()).expect("call");
        assert_eq!(result.is_error, Some(false));
    }

    assert_eq!(
        server.calls(),
        ["initialize", "notifications/initialized", "tools/list", "tools/call", "tools/call"],
        "calls send only tools/call over the connection the list opened"
    );
}

#[test]
fn tool_call_without_a_connection_handshakes_once() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);

    slot.call_tool(&config, params("ping"), &trace()).expect("first call");
    slot.call_tool(&config, params("ping"), &trace()).expect("second call");

    assert_eq!(
        server.calls(),
        ["initialize", "notifications/initialized", "tools/call", "tools/call"]
    );
}

#[test]
fn failed_tool_call_drops_the_connection_without_retrying() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);
    slot.call_tool(&config, params("ping"), &trace()).expect("healthy call");

    server.fail_next("tools/call", 1);
    let failed = slot.call_tool(&config, params("ping"), &trace());
    assert!(matches!(failed, Err(McpError::CallTool(_))), "got {failed:?}");
    slot.call_tool(&config, params("ping"), &trace()).expect("call after reset");

    assert_eq!(
        server.calls(),
        [
            "initialize",
            "notifications/initialized",
            "tools/call",
            "tools/call",
            "DELETE",
            "initialize",
            "notifications/initialized",
            "tools/call",
        ],
        "the failed call is sent once, then the next call handshakes again"
    );
}

#[test]
fn tool_reported_error_keeps_the_connection() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);

    let refused = slot.call_tool(&config, params("refuse"), &trace()).expect("call completes");
    assert_eq!(refused.is_error, Some(true));
    slot.call_tool(&config, params("ping"), &trace()).expect("next call");

    assert_eq!(
        server.calls(),
        ["initialize", "notifications/initialized", "tools/call", "tools/call"],
        "is_error is a tool outcome, not a connection failure"
    );
}
