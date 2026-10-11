//! A real MCP session whose requests carry the client's `_meta` trace / session
//! ids is still served normally: the gate peeks at each body for logging and
//! must hand it on intact.
//!
//! App log lines are not written to a file under test, so the log fields
//! themselves are covered by `request_meta_tests` on the label builder.

use serde_json::{json, Value};

use super::{bearer, http_post_json_auth, issue_live_ticket, start_ephemeral_mcp, SPARK_SLOT};
use crate::agent::diagnostics::{TraceId, TurnId};
use crate::mcp_host::stop_embedded_mcp_runtime;

const SESSION: &str = "spark_chat_5141a723ad70f2cadd4e5885a5421b06";

#[test]
fn requests_carrying_meta_are_served_with_their_bodies_intact() {
    let handle = start_ephemeral_mcp();
    let auth = bearer(&issue_live_ticket(SPARK_SLOT));
    let url = format!("http://127.0.0.1:{}/mcp/spark", handle.local_addr().port());
    let meta = json!({
        "spark/trace_id": TraceId::new().as_str(),
        "spark/turn_id": TurnId::new().as_str(),
        "spark/session_id": SESSION,
    });
    let request = |id: u64, method: &str, params: Value| {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
    };

    let (status, _, session) = http_post_json_auth(
        &url,
        &request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "t", "version": "0" },
                "_meta": meta,
            }),
        ),
        Some(&auth),
        None,
    );
    assert_eq!(status, 200, "initialize with _meta");
    let session = session.expect("initialize returns a session id");

    let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    let (status, _, _) =
        http_post_json_auth(&url, &initialized.to_string(), Some(&auth), Some(&session));
    assert_eq!(status, 202, "notifications/initialized");

    let (status, body, _) = http_post_json_auth(
        &url,
        &request(2, "tools/list", json!({ "_meta": meta })),
        Some(&auth),
        Some(&session),
    );
    assert_eq!(status, 200, "tools/list with _meta");
    assert!(body.contains("get_all_notes_catalog"), "tool list intact: {body}");

    let (status, body, _) = http_post_json_auth(
        &url,
        &request(3, "tools/call", json!({ "name": "no_such_tool", "_meta": meta })),
        Some(&auth),
        Some(&session),
    );
    assert_eq!(status, 200, "tools/call with _meta");
    assert!(body.contains("is not enabled for channel"), "handler reached: {body}");

    stop_embedded_mcp_runtime(handle).expect("stop");
}
