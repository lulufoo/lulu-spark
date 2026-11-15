use super::*;

#[test]
fn board_url_validation_precedes_credentials_and_preserves_request_identity() {
    assert_eq!(diagram_id("https://luluboard.app/view?utm=1#b:b_0123abcd").unwrap(), "b_0123abcd");
    for url in ["https://evil.test/#b:b_0123abcd", "https://secret@luluboard.app/#b:b_0123abcd"] {
        assert_eq!(diagram_id(url), Err("not_board"));
    }
    assert_eq!(diagram_id("https://luluboard.app/#b:b_zzzzzzzz"), Err("invalid_id"));
    assert_eq!(board_graph_request_url("b_0123abcd").unwrap(),
        "https://api.luluboard.app/graphs?diagram_id=b_0123abcd");
    assert!(board_graph_request_url("../bad").is_none());
}

#[test]
fn remote_access_failures_serialize_only_their_scope_without_source_metadata() {
    for (code, status, scope) in [(401, "unauthorized", "credential"),
        (403, "forbidden", "diagram"), (404, "not_found", "diagram")] {
        let failure = read_http(code, r#"{"bmd":"secret","version":7}"#).unwrap_err();
        assert_eq!(serde_json::to_value(failure).unwrap(),
            serde_json::json!({"status":status,"invalidation_scope":scope}));
    }
    for code in [302, 429, 500] {
        assert_eq!(read_http(code, "secret").unwrap_err().status, "network");
        assert_eq!(read_http(code, "secret").unwrap_err().invalidation_scope, None);
    }
}

#[test]
fn invalid_metadata_is_not_an_access_invalidation() {
    let reply=read_http(200, "{}").unwrap_err();
    assert_eq!(reply.status,"invalid_document");
    assert_eq!(reply.invalidation_scope,None);
    assert_eq!(reply.identity,None);
}
