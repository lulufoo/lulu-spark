use super::{
    board_graph_request_url, read_board_sketch_with, BoardSketchReply, FetchOutcome,
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
        r#"{{"bmd":"style abc123\nsketch \"Hi\"\ncontent {token}\n","title":"Hi"}}"#
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
        r#"{"bmd":"boxes \"Plan\"\nbox a \"图\"\n"}"#.into(),
    );
    assert_eq!(boxes.status, "not_sketch");
    assert!(boxes.content_b64.is_none());
    let flow = read_with(&page(""), 200, r#"{"bmd":"flowchart TD\n  a --> b\n"}"#.into());
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
    assert!(!serde_json::to_string(&denied).unwrap().contains(PAT));
    let forbidden = read_with(&page(""), 403, "{}".into());
    assert_eq!(forbidden.status, "unauthorized");
    let missing = read_with(&page(""), 404, "{}".into());
    assert_eq!(missing.status, "not_found");
    let down = read_board_sketch_with(&page(""), |_, _| FetchOutcome::Network);
    assert_eq!(down.status, "network");
    let bad = read_with(
        &page(""),
        200,
        sketch_body("!!!not-base64!!!"),
    );
    assert_eq!(bad.status, "invalid_source");
    assert!(bad.content_b64.is_none());
    vault::delete_board_pat().expect("cleanup");
}
