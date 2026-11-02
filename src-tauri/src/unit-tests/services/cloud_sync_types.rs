use serde_json::{json, Value};

use super::*;

const ID_1: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ID_2: &str = "0123456789abcdef0123456789abcdef";

fn is_bad_response<T: std::fmt::Debug>(result: Result<T, CloudSyncError>) -> bool {
    matches!(result, Err(CloudSyncError::BadResponse(_)))
}

#[test]
fn credentials_are_read_from_the_worker_answer() {
    let got = parse_credentials(&json!({
        "bucket": "spark-abc",
        "accessToken": "gcs-1",
        "expiresAt": 1_800_001_800_i64,
        "extra": true,
    }))
    .expect("credentials");

    assert_eq!(
        got,
        Credentials {
            bucket: "spark-abc".into(),
            access_token: "gcs-1".into(),
            expires_at: 1_800_001_800,
        }
    );
}

#[test]
fn credentials_with_a_missing_or_wrong_field_are_refused() {
    let good = json!({ "bucket": "b", "accessToken": "t", "expiresAt": 5 });
    let broken: [(&str, Value); 8] = [
        ("bucket", json!(null)),
        ("bucket", json!("")),
        ("bucket", json!(7)),
        ("accessToken", json!(null)),
        ("accessToken", json!("")),
        ("expiresAt", json!("5")),
        ("expiresAt", json!(1.5)),
        ("expiresAt", json!(null)),
    ];
    for (field, value) in broken {
        let mut body = good.clone();
        body[field] = value.clone();
        assert!(is_bad_response(parse_credentials(&body)), "{field} = {value}");
    }
    for field in ["bucket", "accessToken", "expiresAt"] {
        let mut body = good.clone();
        body.as_object_mut().unwrap().remove(field);
        assert!(is_bad_response(parse_credentials(&body)), "without {field}");
    }
    assert!(is_bad_response(parse_credentials(&json!([]))));
}

#[test]
fn a_bucket_name_is_only_what_google_allows_because_it_goes_into_a_url() {
    let long_enough = "a".repeat(63);
    let accepted = ["abc", "a1b", "spark-abc", "a.b_c-d", "0abc9", long_enough.as_str()];
    for name in accepted {
        let body = json!({ "bucket": name, "accessToken": "t", "expiresAt": 5 });
        assert!(parse_credentials(&body).is_ok(), "{name}");
    }
    let too_long = "a".repeat(64);
    let refused = [
        "ab",
        too_long.as_str(),
        "-abc",
        "abc-",
        "_abc",
        "abc_",
        ".abc",
        "abc.",
        "Abc",
        "ab c",
        "a/b",
        "a?b",
        "a#b",
        "a%2fb",
        "a\\b",
        "ünï",
        "abc\n",
    ];
    for name in refused {
        let body = json!({ "bucket": name, "accessToken": "t", "expiresAt": 5 });
        assert!(is_bad_response(parse_credentials(&body)), "{name:?}");
    }
}

#[test]
fn an_upload_link_keeps_its_url_and_its_headers_exactly() {
    let got = parse_upload_link(&json!({
        "method": "PUT",
        "url": "https://storage.googleapis.com/b/o?X-Goog-Signature=abc",
        "headers": {
            "x-goog-content-length-range": "0,100",
            "x-goog-if-generation-match": "0",
            "x-goog-meta-version": "3",
        },
        "expiresAt": 1_800_000_600_i64,
        "extra": true,
    }))
    .expect("link");

    assert_eq!(got.url, "https://storage.googleapis.com/b/o?X-Goog-Signature=abc");
    assert_eq!(got.headers.len(), 3);
    assert_eq!(got.headers["x-goog-content-length-range"], "0,100");
    assert_eq!(got.headers["x-goog-if-generation-match"], "0");
    assert_eq!(got.headers["x-goog-meta-version"], "3");

    let none = parse_upload_link(&json!({ "method": "PUT", "url": "u", "headers": {} })).expect("none");
    assert!(none.headers.is_empty());
}

#[test]
fn a_link_that_is_not_a_complete_put_link_is_refused() {
    let good = json!({ "method": "PUT", "url": "u", "headers": { "x-goog-meta-version": "3" } });
    assert!(parse_upload_link(&good).is_ok());

    let broken: [(&str, Value); 10] = [
        ("method", json!("POST")),
        ("method", json!("put")),
        ("method", json!(null)),
        ("url", json!("")),
        ("url", json!(5)),
        ("url", json!(null)),
        ("headers", json!(null)),
        ("headers", json!([])),
        ("headers", json!({ "x-goog-meta-version": 3 })),
        ("headers", json!({ "bad name": "3" })),
    ];
    for (field, value) in broken {
        let mut body = good.clone();
        body[field] = value.clone();
        assert!(is_bad_response(parse_upload_link(&body)), "{field} = {value}");
    }
    let mut body = good.clone();
    body["headers"] = json!({ "x-goog-meta-version": "line\nbreak" });
    assert!(is_bad_response(parse_upload_link(&body)), "a header value with a line break");
    for field in ["method", "url", "headers"] {
        let mut body = good.clone();
        body.as_object_mut().unwrap().remove(field);
        assert!(is_bad_response(parse_upload_link(&body)), "without {field}");
    }
}

#[test]
fn the_upload_link_is_not_printed() {
    let link = parse_upload_link(&json!({
        "method": "PUT",
        "url": "https://storage.googleapis.com/b/o?X-Goog-Signature=link-secret",
        "headers": {},
    }))
    .expect("link");

    assert!(!format!("{link:?}").contains("link-secret"));
}

#[test]
fn only_a_document_id_may_go_into_a_path() {
    assert!(check_document_id(ID_1).is_ok());
    assert!(check_document_id(ID_2).is_ok());
    for id in ["", "aaaa", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "../aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"] {
        assert!(matches!(check_document_id(id), Err(CloudSyncError::InvalidInput(_))), "{id:?}");
    }
}

#[test]
fn the_storage_token_is_not_printed() {
    let credentials = Credentials {
        bucket: "spark-abc".into(),
        access_token: "gcs-secret-token".into(),
        expires_at: 5,
    };

    let printed = format!("{credentials:?}");

    assert!(!printed.contains("gcs-secret-token"));
    assert!(printed.contains("spark-abc"));
}

#[test]
fn documents_are_read_in_the_order_the_worker_sends_them() {
    let got = parse_documents(&json!({
        "documents": [
            { "documentId": ID_2, "version": 0 },
            { "documentId": ID_1, "version": 12, "extra": true },
        ]
    }))
    .expect("documents");

    assert_eq!(
        got,
        [
            CloudDoc { document_id: ID_2.into(), version: 0 },
            CloudDoc { document_id: ID_1.into(), version: 12 },
        ]
    );
    assert_eq!(parse_documents(&json!({ "documents": [] })).expect("empty"), []);
}

#[test]
fn a_document_list_with_one_bad_entry_is_refused_whole() {
    let good = json!({ "documentId": ID_1, "version": 1 });
    let bad_entries = [
        json!({ "documentId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "version": 1 }),
        json!({ "documentId": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "version": 1 }),
        json!({ "documentId": "../aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "version": 1 }),
        json!({ "documentId": "", "version": 1 }),
        json!({ "documentId": 7, "version": 1 }),
        json!({ "version": 1 }),
        json!({ "documentId": ID_1, "version": -1 }),
        json!({ "documentId": ID_1, "version": 1.5 }),
        json!({ "documentId": ID_1, "version": "3" }),
        json!({ "documentId": ID_1 }),
        json!("not an object"),
    ];
    for entry in bad_entries {
        let body = json!({ "documents": [good.clone(), entry.clone()] });
        assert!(is_bad_response(parse_documents(&body)), "{entry}");
    }
}

#[test]
fn a_body_without_a_document_list_is_refused() {
    for body in [
        json!({}),
        json!({ "documents": null }),
        json!({ "documents": {} }),
        json!([]),
    ] {
        assert!(is_bad_response(parse_documents(&body)), "{body}");
    }
}
