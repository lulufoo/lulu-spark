use super::*;

fn cloud(e: CloudSyncError) -> DocSyncError {
    DocSyncError::Cloud(e)
}

#[test]
fn every_error_says_what_went_wrong() {
    let cases = [
        (DocSyncError::Database("no such table".into()), "could not read the document database: no such table"),
        (
            DocSyncError::Pack { doc_id: "abc".into(), message: "raw/x.md: not found".into() },
            "could not pack document abc: raw/x.md: not found",
        ),
        (cloud(CloudSyncError::NotSignedIn), "not signed in"),
        (cloud(CloudSyncError::ObjectChanged), "the document changed in the cloud during the upload"),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
}

#[test]
fn a_cloud_error_converts_without_changing() {
    assert_eq!(DocSyncError::from(CloudSyncError::SessionExpired), cloud(CloudSyncError::SessionExpired));
}

#[test]
fn the_errors_of_one_document_do_not_stop_the_run() {
    let one_document = [
        DocSyncError::Database("x".into()),
        DocSyncError::Pack { doc_id: "a".into(), message: "x".into() },
        cloud(CloudSyncError::InvalidInput("x".into())),
        cloud(CloudSyncError::ObjectTooLarge { limit_bytes: 1 }),
        cloud(CloudSyncError::QuotaExceeded { limit_bytes: 2, used_bytes: 1 }),
        cloud(CloudSyncError::UploadLinkExpired),
        cloud(CloudSyncError::ObjectChanged),
    ];
    for error in one_document {
        assert!(!error.stops_the_run(), "{error:?}");
    }
}

#[test]
fn the_errors_that_hit_every_document_stop_the_run() {
    let every_document = [
        cloud(CloudSyncError::NotSignedIn),
        cloud(CloudSyncError::SessionExpired),
        cloud(CloudSyncError::Unauthorized),
        cloud(CloudSyncError::Network("down".into())),
        cloud(CloudSyncError::Unavailable { status: 503, code: "x".into() }),
        cloud(CloudSyncError::Rejected { status: 403, code: "x".into() }),
        cloud(CloudSyncError::BadResponse("x".into())),
        cloud(CloudSyncError::Vault("x".into())),
        cloud(CloudSyncError::ObjectNotFound),
        cloud(CloudSyncError::StorageTokenRejected),
    ];
    for error in every_document {
        assert!(error.stops_the_run(), "{error:?}");
    }
}
