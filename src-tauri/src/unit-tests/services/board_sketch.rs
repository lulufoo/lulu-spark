use super::{
    board_graph_request_url, parse_document, project_sketch, read_board_sketch_with,
    BoardInvalidationScope, BoardSketchReply, FetchOutcome,
};
use crate::config::vault::{self, test_clear_scope, test_clear_store_fail, test_fail_store};

const PAT: &str = "lbp_do_not_leak_0123456789abcdef";
const ID: &str = "b_0123abcd";
const TOKEN: &str = "PDw+Pz89";

fn page(extra: &str) -> String {
    format!("https://luluboard.app/view?utm=1{extra}#b:{ID}")
}

fn http(status: u16, body: String) -> impl FnOnce(&str, &str) -> FetchOutcome {
    move |pat, id| {
        let url = board_graph_request_url(id).expect("api url");
        assert_eq!(url, format!("https://api.luluboard.app/graphs?diagram_id={ID}"));
        assert!(!url.contains(pat), "{url}");
        FetchOutcome::Http { status, body }
    }
}

fn sketch_body(token: &str) -> String {
    format!(
        r#"{{"bmd":"style abc123\nsketch \"Hi\"\ncontent {token}\n","title":"Hi","version":7}}"#
    )
}

fn read_with(url: &str, status: u16, body: String) -> BoardSketchReply {
    read_board_sketch_with(url, http(status, body))
}

#[test]
fn query_on_a_board_link_returns_only_the_content_token() {
    test_clear_scope();
    vault::set_board_pat(PAT).expect("pat");
    let reply = read_with(&page(""), 200, sketch_body(TOKEN));
    assert_eq!(reply.status, "sketch");
    assert_eq!(reply.content_b64.as_deref(), Some(TOKEN));
    assert_eq!(reply.title.as_deref(), Some("Hi"));
    assert_eq!(reply.version, Some(7));
    assert_eq!(reply.invalidation_scope, None);
    let json = serde_json::to_string(&reply).expect("json");
    assert!(!json.contains(PAT), "{json}");
    assert!(!json.contains("style abc123"), "{json}");
    assert!(!json.contains("sketch \\\"Hi\\\""), "{json}");
    vault::delete_board_pat().expect("cleanup");
}

#[test]
fn non_board_and_illegal_ids_do_not_call_board() {
    let mut calls = 0;
    let miss = read_board_sketch_with("https://evil.test/#b:b_0123abcd", |_, _| {
        calls += 1;
        FetchOutcome::Network
    });
    assert_eq!(miss.status, "not_board");
    let bad = read_board_sketch_with("https://luluboard.app/#b:b_zzzzzzzz", |_, _| {
        calls += 1;
        FetchOutcome::Network
    });
    assert_eq!(bad.status, "invalid_id");
    let userinfo = read_board_sketch_with("https://secret@luluboard.app/#b:b_0123abcd", |_, _| {
        calls += 1;
        FetchOutcome::Network
    });
    assert_eq!(userinfo.status, "not_board");
    assert_eq!(calls, 0);
}

#[test]
fn other_diagram_kinds_stay_links() {
    test_clear_scope();
    vault::set_board_pat(PAT).expect("pat");
    let boxes = read_with(
        &page(""),
        200,
        r#"{"bmd":"boxes \"Plan\"\nbox a \"图\"\n","version":2}"#.into(),
    );
    assert_eq!(boxes.status, "not_sketch");
    assert!(boxes.content_b64.is_none());
    assert_eq!(boxes.version, Some(2));
    assert_eq!(boxes.invalidation_scope, None);
    let flow = read_with(&page(""), 200, r#"{"bmd":"flowchart TD\n  a --> b\n","version":3}"#.into());
    assert_eq!(flow.status, "not_sketch");
    vault::delete_board_pat().expect("cleanup");
}

#[test]
fn missing_pat_and_store_failure_do_not_fetch_or_echo_the_token() {
    test_clear_scope();
    let mut calls = 0;
    let missing = read_board_sketch_with(&page(""), |_, _| {
        calls += 1;
        FetchOutcome::Network
    });
    assert_eq!(missing.status, "unconfigured");
    assert_eq!(missing.invalidation_scope, Some(BoardInvalidationScope::Credential));
    vault::set_board_pat(PAT).expect("pat");
    test_fail_store();
    let failed = read_board_sketch_with(&page(""), |pat, _| {
        calls += 1;
        FetchOutcome::Http {
            status: 200,
            body: pat.to_string(),
        }
    });
    assert_eq!(failed.status, "unavailable");
    assert_eq!(failed.invalidation_scope, None);
    let json = serde_json::to_string(&failed).expect("json");
    assert!(!json.contains(PAT), "{json}");
    assert_eq!(calls, 0);
    test_clear_store_fail();
    vault::delete_board_pat().expect("cleanup");
}

#[test]
fn auth_network_and_illegal_source_are_distinct_and_hide_the_token() {
    test_clear_scope();
    vault::set_board_pat(PAT).expect("pat");
    let denied = read_with(&page(""), 401, format!(r#"{{"error":"{PAT}"}}"#));
    assert_eq!(denied.status, "unauthorized");
    assert_eq!(denied.invalidation_scope, Some(BoardInvalidationScope::Credential));
    assert!(!serde_json::to_string(&denied).unwrap().contains(PAT));
    let forbidden = read_with(&page(""), 403, "{}".into());
    assert_eq!(forbidden.status, "forbidden");
    assert_eq!(forbidden.invalidation_scope, Some(BoardInvalidationScope::Diagram));
    let missing = read_with(&page(""), 404, "{}".into());
    assert_eq!(missing.status, "not_found");
    assert_eq!(missing.invalidation_scope, Some(BoardInvalidationScope::Diagram));
    let down = read_board_sketch_with(&page(""), |_, _| FetchOutcome::Network);
    assert_eq!(down.status, "network");
    assert_eq!(down.invalidation_scope, None);
    let bad = read_with(
        &page(""),
        200,
        sketch_body("!!!not-base64!!!"),
    );
    assert_eq!(bad.status, "invalid_source");
    assert!(bad.content_b64.is_none());
    assert_eq!(bad.version, Some(7));
    assert_eq!(bad.invalidation_scope, None);
    vault::delete_board_pat().expect("cleanup");
}

#[test]
fn remote_access_failures_serialize_only_their_scope_without_source_metadata() {
    for (code, status, scope) in [
        (401, "unauthorized", "credential"),
        (403, "forbidden", "diagram"),
        (404, "not_found", "diagram"),
    ] {
        let failure = super::interpret(code, &sketch_body(TOKEN));
        assert_eq!(
            serde_json::to_value(failure).expect("wire reply"),
            serde_json::json!({ "status": status, "invalidation_scope": scope })
        );
    }
    for code in [302, 429, 500] {
        assert_eq!(
            serde_json::to_value(super::interpret(code, &sketch_body(TOKEN))).expect("wire reply"),
            serde_json::json!({ "status": "network" })
        );
    }
}

#[test]
fn source_versions_must_be_positive_database_integers_without_a_default() {
    for version in [1, 7, i32::MAX] {
        let body = serde_json::json!({"bmd":"sketch\ncontent PDw+Pz89", "version": version});
        let doc = parse_document(&body.to_string()).expect("valid integer");
        assert_eq!(doc.version, version);
        assert_eq!(project_sketch(&doc).version, Some(version));
    }
    for version in [
        serde_json::Value::Null,
        serde_json::json!(0),
        serde_json::json!(-1),
        serde_json::json!(1.0),
        serde_json::json!(1.5),
        serde_json::json!("1"),
        serde_json::json!(true),
        serde_json::json!(i64::from(i32::MAX) + 1),
        serde_json::json!(u64::MAX),
    ] {
        let body = serde_json::json!({"bmd":"sketch\ncontent PDw+Pz89", "version": version});
        let failure = parse_document(&body.to_string()).expect_err("invalid version");
        assert_eq!(failure.status, "invalid_document");
        assert_eq!(failure.version, None);
        assert_eq!(failure.invalidation_scope, None);
    }
    assert_eq!(
        parse_document(r#"{"bmd":"sketch\ncontent PDw+Pz89"}"#)
            .expect_err("missing version").status,
        "invalid_document"
    );
}

#[test]
fn source_metadata_failures_are_separate_from_preview_failures() {
    for body in [
        "invalid JSON",
        "[]",
        r#"{"version":1}"#,
        r#"{"version":1,"bmd":null}"#,
        r#"{"version":1,"bmd":"boxes","title":42}"#,
    ] {
        let failure = parse_document(body).expect_err("invalid source metadata");
        assert_eq!(failure.status, "invalid_document");
        assert_eq!(failure.version, None);
        assert_eq!(failure.invalidation_scope, None);
    }
    let doc = parse_document(&sketch_body("not-base64!")).expect("valid Board document");
    assert!(doc.bmd.contains("not-base64!"));
    let projection = project_sketch(&doc);
    assert_eq!(projection.status, "invalid_source");
    assert_eq!(projection.version, Some(7));
    assert_eq!(projection.invalidation_scope, None);
    assert!(doc.bmd.contains("not-base64!"), "projection must not change source");
}

#[test]
fn generic_source_is_preserved_without_type_filtering_or_renderer_requirements() {
    for bmd in [
        "boxes \"Plan\"\r\nbox a \"图\"\n",
        "flowchart TD\n a --> b\n",
        "a future source type",
        "",
    ] {
        let body = serde_json::json!({"bmd":bmd,"title":"  原始标题  ","version":4});
        let doc = parse_document(&body.to_string()).expect("generic source");
        assert_eq!(doc.bmd, bmd);
        assert_eq!(doc.title, "  原始标题  ");
        let projection = project_sketch(&doc);
        assert_eq!(projection.status, "not_sketch");
        assert_eq!(projection.title.as_deref(), Some("原始标题"));
        assert_eq!(projection.version, Some(4));
        assert_eq!(projection.invalidation_scope, None);
    }
}
