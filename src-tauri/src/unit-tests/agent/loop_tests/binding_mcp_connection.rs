//! Binding Reset releases the Host Agent's live MCP connection.

use super::support::*;
use crate::agent::diagnostics::TurnId;
use crate::agent::mcp::{self, McpTurn, ResetReason};

// Shared fixture compiled into several test modules; each uses a subset.
#[allow(dead_code)]
#[path = "../mcp_wire_server.rs"]
mod wire_server;

use wire_server::WireServer;

#[test]
fn reset_binding_closes_the_live_mcp_connection_and_the_next_list_handshakes() {
    with_sandbox(|| {
        let server = WireServer::start();
        let config = server.config(&[]);
        mcp::discover_tools(&config, &McpTurn::new(&TurnId::new(), "")).expect("list over the live connection");

        r#loop::reset_binding().expect("reset");

        assert_eq!(
            server.calls(),
            ["initialize", "notifications/initialized", "tools/list", "DELETE"],
            "Reset closes the server session"
        );

        mcp::discover_tools(&config, &McpTurn::new(&TurnId::new(), "")).expect("list after reset");
        assert_eq!(
            server.calls()[4..],
            ["initialize", "notifications/initialized", "tools/list"],
            "the next request handshakes on a new connection"
        );
        mcp::release_connection(ResetReason::BindingReset);
    });
}

#[test]
fn second_reset_has_no_connection_left_to_close() {
    with_sandbox(|| {
        let server = WireServer::start();
        mcp::discover_tools(&server.config(&[]), &McpTurn::new(&TurnId::new(), "")).expect("list");

        r#loop::reset_binding().expect("first reset");
        r#loop::reset_binding().expect("second reset");

        let deletes = server.calls().iter().filter(|call| *call == "DELETE").count();
        assert_eq!(deletes, 1, "one session, one DELETE");
    });
}
