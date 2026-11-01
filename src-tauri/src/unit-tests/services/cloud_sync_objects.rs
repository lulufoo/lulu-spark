use std::io::Cursor;

use serde_json::json;

use super::*;
use crate::services::cloud_sync::tests::{
    creds_reply, expired_reply, link_reply, xml_error, Reply, Rig, DOC_A, JWT, NOW,
};

/// Every byte value, so a package that is not text cannot pass by luck.
fn package() -> Vec<u8> {
    (0..=255u8).cycle().take(3000).collect()
}

fn put_path(rig: &Rig, index: usize) -> String {
    rig.worker.recorded()[index].path.clone()
}

// --- upload ---

#[test]
fn an_upload_asks_the_worker_for_a_link_and_sends_the_bytes_to_it() {
    let rig = Rig::new(vec![link_reply("/put/one"), Reply::raw(200, "")]);

    rig.sync.upload(DOC_A, &package(), 3).expect("upload");

    let requests = rig.worker.recorded();
    assert_eq!(requests.len(), 2);
    let ask = &requests[0];
    assert_eq!((ask.method.as_str(), ask.path.as_str()), ("POST", "/uploads"));
    assert_eq!(ask.header("authorization"), Some(format!("Bearer {JWT}").as_str()));
    assert_eq!(ask.json(), json!({ "documentId": DOC_A, "size": 3000, "version": 3 }));

    let put = &requests[1];
    assert_eq!(put.method, "PUT");
    assert_eq!(put.path, "/put/one?X-Goog-Signature=abc");
    assert_eq!(put.body, package());
    // The headers of the link, as they came: Google checks them against the signature.
    assert_eq!(put.header("x-goog-content-length-range"), Some("0,1000000"));
    assert_eq!(put.header("x-goog-if-generation-match"), Some("0"));
    assert_eq!(put.header("x-goog-meta-version"), Some("3"));
    assert_eq!(put.header("content-type"), Some("application/zip"));
    // The link carries the permission. The session token stays with the Worker.
    assert_eq!(put.header("authorization"), None);
}

#[test]
fn a_link_that_expired_on_the_way_is_replaced_by_a_new_one_once() {
    let rig = Rig::new(vec![
        link_reply("/put/one"),
        expired_reply(),
        link_reply("/put/two"),
        Reply::raw(200, ""),
    ]);

    rig.sync.upload(DOC_A, &package(), 3).expect("upload");

    let paths: Vec<String> = (0..4).map(|i| put_path(&rig, i)).collect();
    assert_eq!(
        paths,
        [
            "/uploads",
            "/put/one?X-Goog-Signature=abc",
            "/uploads",
            "/put/two?X-Goog-Signature=abc"
        ]
    );
}

#[test]
fn a_link_that_expires_again_is_an_error_and_there_is_no_third_try() {
    let rig = Rig::new(vec![
        link_reply("/put/one"),
        expired_reply(),
        link_reply("/put/two"),
        expired_reply(),
        link_reply("/put/three"),
        Reply::raw(200, ""),
    ]);

    let err = rig.sync.upload(DOC_A, &package(), 3).unwrap_err();

    assert_eq!(err, CloudSyncError::UploadLinkExpired);
    assert_eq!(rig.worker.recorded().len(), 4);
}

#[test]
fn google_answers_to_the_bytes_become_clear_errors_and_are_not_retried() {
    let cases = [
        (xml_error(412, "PreconditionFailed"), CloudSyncError::ObjectChanged),
        (Reply::raw(412, ""), CloudSyncError::ObjectChanged),
        (
            xml_error(403, "SignatureDoesNotMatch"),
            CloudSyncError::Rejected { status: 403, code: "SignatureDoesNotMatch".into() },
        ),
        (
            xml_error(400, "EntityTooLarge"),
            CloudSyncError::Rejected { status: 400, code: "EntityTooLarge".into() },
        ),
        (
            Reply::raw(400, "bad"),
            CloudSyncError::Rejected { status: 400, code: "unknown".into() },
        ),
        (
            Reply::raw(400, "<Code>not a plain name!</Code>"),
            CloudSyncError::Rejected { status: 400, code: "unknown".into() },
        ),
        (
            xml_error(503, "ServiceUnavailable"),
            CloudSyncError::Unavailable { status: 503, code: "ServiceUnavailable".into() },
        ),
        (
            Reply::raw(429, ""),
            CloudSyncError::Unavailable { status: 429, code: "unknown".into() },
        ),
        (
            Reply::json(403, json!({ "error": { "errors": [{ "reason": "forbidden" }] } })),
            CloudSyncError::Rejected { status: 403, code: "forbidden".into() },
        ),
        (
            xml_error(403, "Some_Code1"),
            CloudSyncError::Rejected { status: 403, code: "Some_Code1".into() },
        ),
        (
            xml_error(403, &"a".repeat(64)),
            CloudSyncError::Rejected { status: 403, code: "a".repeat(64) },
        ),
        (
            xml_error(403, &"a".repeat(65)),
            CloudSyncError::Rejected { status: 403, code: "unknown".into() },
        ),
        (
            xml_error(403, ""),
            CloudSyncError::Rejected { status: 403, code: "unknown".into() },
        ),
        (
            Reply::raw(403, "<Code>no end"),
            CloudSyncError::Rejected { status: 403, code: "unknown".into() },
        ),
        (
            Reply::raw(300, ""),
            CloudSyncError::BadResponse("unexpected status 300".into()),
        ),
    ];
    for (reply, expected) in cases {
        let rig = Rig::new(vec![link_reply("/put/one"), reply.clone()]);
        assert_eq!(rig.sync.upload(DOC_A, &package(), 3).unwrap_err(), expected, "{reply:?}");
        assert_eq!(rig.worker.recorded().len(), 2, "{reply:?}");
    }
}

#[test]
fn a_link_that_points_anywhere_but_google_is_refused_and_no_bytes_are_sent() {
    let outside = [
        "https://evil.example/put/one",
        "http://127.0.0.1:1/put/one",
        "{mock}@evil.example/put/one",
        "{mock}.evil.example/put/one",
        "{mock}",
    ];
    for url in outside {
        let reply = Reply::json(
            200,
            json!({ "method": "PUT", "url": url, "headers": {}, "expiresAt": NOW + 600 }),
        );
        let rig = Rig::new(vec![reply, Reply::raw(200, "")]);

        let err = rig.sync.upload(DOC_A, &package(), 3).unwrap_err();

        assert!(matches!(err, CloudSyncError::BadResponse(_)), "{url}: {err:?}");
        assert_eq!(rig.worker.recorded().len(), 1, "{url}");
    }
}

#[test]
fn a_link_answer_that_is_not_a_put_link_is_a_bad_response() {
    let answers = [
        json!({ "method": "POST", "url": "{mock}/p", "headers": {} }),
        json!({ "url": "{mock}/p", "headers": {} }),
        json!({ "method": "PUT", "headers": {} }),
        json!({ "method": "PUT", "url": "{mock}/p" }),
        json!({ "method": "PUT", "url": "{mock}/p", "headers": { "x-goog-meta-version": 3 } }),
        json!({ "method": "PUT", "url": "{mock}/p", "headers": { "bad name": "1" } }),
        json!({ "method": "PUT", "url": "{mock}/p", "headers": { "x": "line\nbreak" } }),
        json!([]),
    ];
    for answer in answers {
        let rig = Rig::new(vec![Reply::json(200, answer.clone()), Reply::raw(200, "")]);
        let err = rig.sync.upload(DOC_A, &package(), 3).unwrap_err();
        assert!(matches!(err, CloudSyncError::BadResponse(_)), "{answer}: {err:?}");
        assert_eq!(rig.worker.recorded().len(), 1, "{answer}");
    }
}

#[test]
fn an_upload_that_cannot_be_a_request_is_refused_before_anything_is_sent() {
    let rig = Rig::new(vec![]);
    let bad_ids = ["", "short", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "../aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"];

    for id in bad_ids {
        let err = rig.sync.upload(id, &package(), 3).unwrap_err();
        assert!(matches!(err, CloudSyncError::InvalidInput(_)), "{id}: {err:?}");
    }
    assert!(matches!(rig.sync.upload(DOC_A, &[], 3), Err(CloudSyncError::InvalidInput(_))));
    assert!(matches!(rig.sync.upload(DOC_A, &package(), -1), Err(CloudSyncError::InvalidInput(_))));
    assert_eq!(rig.worker.recorded().len(), 0);
}

#[test]
fn an_upload_without_a_usable_session_sends_nothing() {
    let rig = Rig::new(vec![]);
    rig.sign_out();
    assert_eq!(rig.sync.upload(DOC_A, &package(), 3).unwrap_err(), CloudSyncError::NotSignedIn);

    rig.sign_in("user-1", JWT, NOW);
    assert_eq!(rig.sync.upload(DOC_A, &package(), 3).unwrap_err(), CloudSyncError::SessionExpired);
    assert_eq!(rig.worker.recorded().len(), 0);
}

// --- read ---

#[test]
fn a_document_is_read_with_the_storage_token() {
    let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 1800), Reply::bytes(200, &package())]);

    let bytes = rig.sync.read(DOC_A).expect("read");

    assert_eq!(bytes, package());
    let requests = rig.worker.recorded();
    assert_eq!(requests[0].path, "/credentials");
    assert_eq!(requests[1].method, "GET");
    assert_eq!(requests[1].path, format!("/storage/v1/b/spark-abc/o/{DOC_A}?alt=media"));
    assert_eq!(requests[1].header("authorization"), Some("Bearer gcs-1"));
}

#[test]
fn reads_share_the_stored_token() {
    let rig = Rig::new(vec![
        creds_reply("gcs-1", NOW + 1800),
        Reply::raw(200, "one"),
        Reply::raw(200, "two"),
    ]);

    assert_eq!(rig.sync.read(DOC_A).expect("first"), b"one");
    assert_eq!(rig.sync.read(DOC_A).expect("second"), b"two");

    assert_eq!(rig.worker.recorded().len(), 3);
}

#[test]
fn a_token_google_refuses_is_replaced_once_and_the_read_is_tried_again() {
    let rig = Rig::new(vec![
        creds_reply("gcs-1", NOW + 1800),
        Reply::raw(401, ""),
        creds_reply("gcs-2", NOW + 1800),
        Reply::raw(200, "the bytes"),
    ]);

    assert_eq!(rig.sync.read(DOC_A).expect("read"), b"the bytes");

    let requests = rig.worker.recorded();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[1].header("authorization"), Some("Bearer gcs-1"));
    assert_eq!(requests[3].header("authorization"), Some("Bearer gcs-2"));
}

#[test]
fn a_token_google_refuses_twice_is_an_error_and_there_is_no_third_try() {
    let rig = Rig::new(vec![
        creds_reply("gcs-1", NOW + 1800),
        Reply::raw(401, ""),
        creds_reply("gcs-2", NOW + 1800),
        Reply::raw(401, ""),
        creds_reply("gcs-3", NOW + 1800),
        Reply::raw(200, "never read"),
    ]);

    assert_eq!(rig.sync.read(DOC_A).unwrap_err(), CloudSyncError::StorageTokenRejected);
    assert_eq!(rig.worker.recorded().len(), 4);
}

#[test]
fn google_answers_to_a_read_become_clear_errors() {
    let cases = [
        (Reply::raw(404, ""), CloudSyncError::ObjectNotFound),
        (
            Reply::json(403, json!({ "error": { "errors": [{ "reason": "forbidden" }] } })),
            CloudSyncError::Rejected { status: 403, code: "forbidden".into() },
        ),
        (
            Reply::raw(500, "oops"),
            CloudSyncError::Unavailable { status: 500, code: "unknown".into() },
        ),
        (
            Reply::raw(429, ""),
            CloudSyncError::Unavailable { status: 429, code: "unknown".into() },
        ),
    ];
    for (reply, expected) in cases {
        let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 1800), reply.clone()]);
        assert_eq!(rig.sync.read(DOC_A).unwrap_err(), expected, "{reply:?}");
        assert_eq!(rig.worker.recorded().len(), 2, "{reply:?}");
    }
}

#[test]
fn a_read_that_cannot_be_a_request_or_has_no_session_sends_nothing() {
    let rig = Rig::new(vec![]);
    assert!(matches!(rig.sync.read("../etc/passwd"), Err(CloudSyncError::InvalidInput(_))));
    assert!(matches!(rig.sync.read(""), Err(CloudSyncError::InvalidInput(_))));

    rig.sign_out();
    assert_eq!(rig.sync.read(DOC_A).unwrap_err(), CloudSyncError::NotSignedIn);
    assert_eq!(rig.worker.recorded().len(), 0);
}

#[test]
fn a_read_never_follows_a_redirect_with_the_token() {
    let rig = Rig::new(vec![
        creds_reply("gcs-1", NOW + 1800),
        Reply::redirect("/elsewhere"),
        Reply::raw(200, "from elsewhere"),
    ]);

    let err = rig.sync.read(DOC_A).unwrap_err();

    assert!(matches!(err, CloudSyncError::BadResponse(_)), "{err:?}");
    assert_eq!(rig.worker.recorded().len(), 2);
}

#[test]
fn an_answer_is_read_up_to_the_cap_and_no_further() {
    let exactly = vec![7u8; 10];
    assert_eq!(read_capped(Cursor::new(&exactly), 10).expect("at the cap"), exactly);
    assert_eq!(read_capped(Cursor::new(&exactly[..9]), 10).expect("under the cap").len(), 9);
    let err = read_capped(Cursor::new(vec![7u8; 11]), 10).unwrap_err();
    assert!(matches!(err, CloudSyncError::BadResponse(_)), "{err:?}");
}

#[test]
fn the_download_cap_is_as_big_as_what_doc_zip_unpacks() {
    assert_eq!(MAX_DOWNLOAD_BYTES, crate::services::doc_zip::MAX_UNPACKED_BYTES);
}
