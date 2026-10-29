use axum::http::{header, HeaderMap, HeaderValue};
use serde_json::{json, Map, Value};

use super::{
    bearer_fingerprint, clean_label, diagnose_bearer, AuthDiagnosis, AuthFailure, Caller,
};
use crate::services::mcp_oauth::{LedgerRecord, OAuthError, Slot, TicketHandle, TicketState};

const SECRET: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn headers(pairs: &[(header::HeaderName, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.insert(name.clone(), HeaderValue::from_str(value).expect("header value"));
    }
    map
}

fn bearer(secret: &str) -> String {
    format!("Bearer {secret}")
}

fn fields(caller: &Caller) -> Value {
    let mut object = Map::new();
    caller.add_to(&mut object);
    Value::Object(object)
}

fn record(state: TicketState, secret: &str) -> LedgerRecord {
    LedgerRecord {
        slot: Slot::Codex,
        handle: TicketHandle::from_secret(secret),
        state,
    }
}

fn diagnose(presented: Option<&str>, stored: Result<Option<LedgerRecord>, OAuthError>) -> AuthDiagnosis {
    diagnose_bearer(Ok(Slot::Codex), presented, stored)
}

#[test]
fn fingerprint_is_the_first_eight_hex_of_sha256() {
    // SHA-256("abc") = ba7816bf 8f01cfea ...
    assert_eq!(bearer_fingerprint("abc"), "ba7816bf");
    let fingerprint = bearer_fingerprint(SECRET);
    assert_eq!(fingerprint.len(), 8);
    assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(fingerprint, bearer_fingerprint(SECRET), "stable for one Bearer");
    assert_ne!(fingerprint, bearer_fingerprint(&format!("{SECRET}0")), "a new Bearer changes it");
}

#[test]
fn caller_fields_carry_slot_user_agent_and_fingerprint_but_never_the_bearer() {
    let caller = Caller::from_headers(
        "spark",
        &headers(&[
            (header::AUTHORIZATION, &bearer(SECRET)),
            (header::USER_AGENT, "cursor-vscode/1.2"),
        ]),
    );
    let logged = fields(&caller);

    assert_eq!(logged["slot"], "spark");
    assert_eq!(logged["user_agent"], "cursor-vscode/1.2");
    assert_eq!(logged["bearer_fp"], bearer_fingerprint(SECRET));
    let text = logged.to_string();
    assert!(!text.contains(SECRET), "no plaintext: {text}");
    assert!(!text.contains(&SECRET[..8]), "no prefix: {text}");
    assert!(!text.contains(&SECRET[SECRET.len() - 8..]), "no suffix: {text}");
}

#[test]
fn caller_without_bearer_or_user_agent_logs_only_the_slot() {
    let logged = fields(&Caller::from_headers("cursor_ide", &HeaderMap::new()));
    assert_eq!(logged, json!({ "slot": "cursor_ide" }));
}

#[test]
fn bearer_header_without_a_secret_has_no_fingerprint() {
    let caller = Caller::from_headers("spark", &headers(&[(header::AUTHORIZATION, "Bearer ")]));
    assert!(fields(&caller).get("bearer_fp").is_none());
}

#[test]
fn labels_lose_control_characters_and_are_cut_to_length() {
    assert_eq!(clean_label("a\r\nb\tc", 20).as_deref(), Some("abc"));
    assert_eq!(clean_label(&"x".repeat(500), 80).map(|s| s.len()), Some(80));
    assert_eq!(clean_label(" \n\t ", 80), None);

    let caller = Caller::from_headers(
        "spark",
        &headers(&[(header::USER_AGENT, "agent/1\t.0")]),
    );
    assert_eq!(fields(&caller)["user_agent"], "agent/1.0");
}

#[test]
fn diagnosis_names_each_failure() {
    let live = record(TicketState::Live, "current-ticket-secret");
    let revoked = record(TicketState::Revoked, "current-ticket-secret");

    let missing = diagnose(None, Ok(Some(live.clone())));
    assert_eq!((missing.reason, missing.live_fp), (AuthFailure::MissingBearer, None));

    let unknown = diagnose_bearer(Err(OAuthError::slot_unknown), Some("x"), Ok(None));
    assert_eq!(unknown.reason, AuthFailure::SlotUnknown);

    let store = diagnose(Some("x"), Err(OAuthError::store_unavailable));
    assert_eq!(store.reason, AuthFailure::StoreUnavailable);

    let nothing_issued = diagnose(Some("x"), Ok(None));
    assert_eq!((nothing_issued.reason, nothing_issued.live_fp), (AuthFailure::TicketMismatch, None));

    let gone = diagnose(Some("current-ticket-secret"), Ok(Some(revoked)));
    assert_eq!((gone.reason, gone.live_fp), (AuthFailure::TicketRevoked, None));
}

#[test]
fn a_stale_bearer_is_told_apart_from_the_current_ticket_by_fingerprint() {
    let live = record(TicketState::Live, "rotated-in-ticket-secret");
    let diagnosis = diagnose(Some("rotated-away-ticket-secret"), Ok(Some(live)));

    assert_eq!(diagnosis.reason, AuthFailure::TicketMismatch);
    let live_fp = diagnosis.live_fp.expect("a Live ticket exists");
    assert_eq!(live_fp, bearer_fingerprint("rotated-in-ticket-secret"));
    assert_ne!(live_fp, bearer_fingerprint("rotated-away-ticket-secret"));
}

#[test]
fn diagnosis_fields_never_contain_a_secret() {
    let diagnosis = diagnose(
        Some("rotated-away-ticket-secret"),
        Ok(Some(record(TicketState::Live, "rotated-in-ticket-secret"))),
    );
    let mut object = Map::new();
    diagnosis.add_to(&mut object);
    let text = Value::Object(object.clone()).to_string();

    assert_eq!(object["auth_reason"], "ticket_mismatch");
    assert!(object["live_fp"].is_string());
    assert!(!text.contains("secret"), "{text}");
}
