use serde_json::json;

use super::*;
use crate::services::cloud_sync::tests::{
    creds_reply, dead_url, list_reply, seen, Reply, Rig, DOC_A, DOC_B, JWT, NOW, USER,
};
use crate::services::cloud_sync::tests::MockWorker;

fn token(rig: &Rig) -> String {
    rig.sync.credentials().expect("credentials").access_token
}

// --- the app wiring ---

#[test]
fn the_app_talks_to_the_deployed_worker_by_the_system_clock() {
    assert_eq!(CloudSync::production().base_url, WORKER_URL);
    let system = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    assert!((unix_now() - system).abs() <= 5);
}

// --- exchanging the session for a storage token ---

#[test]
fn the_session_token_is_exchanged_for_a_storage_token() {
    let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 1800)]);

    let got = rig.sync.credentials().expect("credentials");

    assert_eq!(
        got,
        Credentials {
            bucket: "spark-abc".into(),
            access_token: "gcs-1".into(),
            expires_at: NOW + 1800,
        }
    );
    assert_eq!(rig.worker.seen(), [seen("POST", "/credentials", JWT)]);
}

#[test]
fn a_stored_token_is_reused_until_shortly_before_it_expires() {
    let expires_at = NOW + 1800;
    let rig = Rig::new(vec![
        creds_reply("gcs-1", expires_at),
        creds_reply("gcs-2", expires_at + 1800),
    ]);
    assert_eq!(token(&rig), "gcs-1");

    rig.at(expires_at - REFRESH_MARGIN_SECS - 1);
    assert_eq!(token(&rig), "gcs-1");
    assert_eq!(rig.worker.seen().len(), 1);

    rig.at(expires_at - REFRESH_MARGIN_SECS);
    assert_eq!(token(&rig), "gcs-2");
    assert_eq!(rig.worker.seen().len(), 2);
}

#[test]
fn a_short_lived_token_is_exchanged_again_on_every_call() {
    let rig = Rig::new(vec![
        creds_reply("gcs-1", NOW + 10),
        creds_reply("gcs-2", NOW + 10),
        creds_reply("gcs-3", NOW + 10),
    ]);

    assert_eq!(
        [token(&rig), token(&rig), token(&rig)],
        ["gcs-1", "gcs-2", "gcs-3"]
    );
    assert_eq!(rig.worker.seen().len(), 3);
}

#[test]
fn a_token_stored_for_one_user_is_not_given_to_another() {
    let rig = Rig::new(vec![
        creds_reply("gcs-of-user-1", NOW + 1800),
        creds_reply("gcs-of-user-2", NOW + 1800),
    ]);
    assert_eq!(token(&rig), "gcs-of-user-1");

    rig.sign_in("user-2", "jwt-2", NOW + 3600);

    assert_eq!(token(&rig), "gcs-of-user-2");
    assert_eq!(
        rig.worker.seen(),
        [
            seen("POST", "/credentials", JWT),
            seen("POST", "/credentials", "jwt-2")
        ]
    );
}

#[test]
fn a_refreshed_session_of_the_same_user_keeps_the_stored_token() {
    let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 1800)]);
    assert_eq!(token(&rig), "gcs-1");

    rig.sign_in(USER, "jwt-refreshed", NOW + 7200);

    assert_eq!(token(&rig), "gcs-1");
    assert_eq!(rig.worker.seen().len(), 1);
}

#[test]
fn a_failed_exchange_is_tried_again_by_the_next_call() {
    let rig = Rig::new(vec![
        Reply::json(503, json!({ "error": "storage_unavailable" })),
        creds_reply("gcs-1", NOW + 1800),
    ]);

    assert!(rig.sync.credentials().is_err());

    assert_eq!(token(&rig), "gcs-1");
    assert_eq!(rig.worker.seen().len(), 2);
}

// --- listing the cloud documents ---

#[test]
fn the_cloud_documents_come_back_as_id_and_version() {
    let rig = Rig::new(vec![list_reply(&[(DOC_A, 3), (DOC_B, 0)]), list_reply(&[])]);

    let documents = rig.sync.list_documents().expect("documents");
    assert_eq!(
        documents,
        [
            CloudDoc { document_id: DOC_A.into(), version: 3 },
            CloudDoc { document_id: DOC_B.into(), version: 0 },
        ]
    );
    assert_eq!(rig.sync.list_documents().expect("empty"), []);
    assert_eq!(
        rig.worker.seen(),
        [seen("GET", "/documents", JWT), seen("GET", "/documents", JWT)]
    );
}

#[test]
fn a_slash_at_the_end_of_the_address_is_ignored() {
    let worker = MockWorker::start(vec![list_reply(&[])]);
    let url = format!("{}/", worker.url);
    let rig = Rig::with_url(worker, &url);

    rig.sync.list_documents().expect("documents");

    assert_eq!(rig.worker.seen(), [seen("GET", "/documents", JWT)]);
}

// --- no usable session ---

#[test]
fn without_a_session_the_worker_is_not_called() {
    let rig = Rig::new(vec![]);
    rig.sign_out();

    assert_eq!(rig.sync.credentials().unwrap_err(), CloudSyncError::NotSignedIn);
    assert_eq!(rig.sync.list_documents().unwrap_err(), CloudSyncError::NotSignedIn);
    assert_eq!(rig.worker.seen(), []);
}

#[test]
fn signing_out_stops_the_use_of_a_stored_token() {
    let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 1800)]);
    assert_eq!(token(&rig), "gcs-1");

    rig.sign_out();

    assert_eq!(rig.sync.credentials().unwrap_err(), CloudSyncError::NotSignedIn);
}

#[test]
fn an_expired_session_is_refused_before_any_call() {
    let rig = Rig::new(vec![list_reply(&[])]);

    for expires_at in [NOW - 1, NOW] {
        rig.sign_in(USER, JWT, expires_at);
        assert_eq!(rig.sync.credentials().unwrap_err(), CloudSyncError::SessionExpired);
        assert_eq!(rig.sync.list_documents().unwrap_err(), CloudSyncError::SessionExpired);
    }
    assert_eq!(rig.worker.seen(), []);

    rig.sign_in(USER, JWT, NOW + 1);
    rig.sync.list_documents().expect("a session with one second left is still good");
}

#[test]
fn an_expired_session_stops_the_use_of_a_stored_token() {
    let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 7200)]);
    assert_eq!(token(&rig), "gcs-1");

    rig.at(NOW + 3600);

    assert_eq!(rig.sync.credentials().unwrap_err(), CloudSyncError::SessionExpired);
}

#[test]
fn a_vault_that_cannot_be_read_is_reported() {
    let rig = Rig::new(vec![]);
    rig.vault_fails("locked");

    let expected = CloudSyncError::Vault("store: locked".into());
    assert_eq!(rig.sync.credentials().unwrap_err(), expected);
    assert_eq!(rig.sync.list_documents().unwrap_err(), expected);
    assert_eq!(rig.worker.seen(), []);
}

// --- what the Worker can answer ---

fn unavailable(status: u16, code: &str) -> CloudSyncError {
    CloudSyncError::Unavailable { status, code: code.into() }
}

fn rejected(status: u16, code: &str) -> CloudSyncError {
    CloudSyncError::Rejected { status, code: code.into() }
}

#[test]
fn every_kind_of_worker_failure_is_a_clear_error_on_both_calls() {
    let cases = [
        (Reply::json(401, json!({ "error": "unauthorized" })), CloudSyncError::Unauthorized),
        (Reply::raw(401, ""), CloudSyncError::Unauthorized),
        (
            Reply::json(503, json!({ "error": "auth_unavailable" })),
            unavailable(503, "auth_unavailable"),
        ),
        (
            Reply::json(503, json!({ "error": "storage_unavailable" })),
            unavailable(503, "storage_unavailable"),
        ),
        (Reply::raw(500, "oops"), unavailable(500, "unknown")),
        (Reply::raw(502, "<html>bad gateway</html>"), unavailable(502, "unknown")),
        (Reply::raw(599, ""), unavailable(599, "unknown")),
        (
            Reply::json(400, json!({ "error": "invalid_request", "detail": "x" })),
            rejected(400, "invalid_request"),
        ),
        (Reply::json(404, json!({ "error": "not_found" })), rejected(404, "not_found")),
        (
            Reply::json(409, json!({ "error": "bucket_name_taken" })),
            rejected(409, "bucket_name_taken"),
        ),
        (Reply::json(403, json!({ "error": 7 })), rejected(403, "unknown")),
    ];
    for (reply, expected) in cases {
        let rig = Rig::new(vec![reply.clone(), reply.clone()]);
        assert_eq!(rig.sync.credentials().unwrap_err(), expected, "credentials, {reply:?}");
        assert_eq!(rig.sync.list_documents().unwrap_err(), expected, "list, {reply:?}");
    }
}

#[test]
fn a_redirect_is_not_followed() {
    let rig = Rig::new(vec![Reply::redirect("/documents"), list_reply(&[])]);

    let err = rig.sync.list_documents().unwrap_err();

    assert!(matches!(err, CloudSyncError::BadResponse(_)), "{err:?}");
    assert_eq!(rig.worker.seen().len(), 1);
}

#[test]
fn an_answer_that_is_not_what_the_worker_promises_is_a_bad_response() {
    let rig = Rig::new(vec![
        Reply::raw(200, "not json"),
        Reply::json(200, json!({})),
        Reply::json(200, json!({ "documents": [{ "documentId": "x", "version": 1 }] })),
    ]);

    for result in [
        rig.sync.credentials().map(drop),
        rig.sync.credentials().map(drop),
        rig.sync.list_documents().map(drop),
    ] {
        assert!(matches!(result, Err(CloudSyncError::BadResponse(_))), "{result:?}");
    }
}

#[test]
fn an_unreachable_worker_is_a_network_error_that_does_not_leak_the_token() {
    let rig = Rig::with_url(MockWorker::start(vec![]), &dead_url());

    for err in [
        rig.sync.credentials().unwrap_err(),
        rig.sync.list_documents().unwrap_err(),
    ] {
        let CloudSyncError::Network(why) = &err else {
            panic!("not a network error: {err:?}");
        };
        assert!(why.contains("tcp connect error"), "the cause is part of the text: {why}");
        assert!(!format!("{err:?} {err}").contains(JWT));
    }
}
