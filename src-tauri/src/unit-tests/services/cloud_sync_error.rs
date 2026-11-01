use super::*;

#[test]
fn every_error_says_what_went_wrong() {
    let cases = [
        (CloudSyncError::NotSignedIn, "not signed in"),
        (CloudSyncError::SessionExpired, "sign-in session expired"),
        (
            CloudSyncError::Unauthorized,
            "cloud sync refused the sign-in session (401)",
        ),
        (
            CloudSyncError::Network("timed out".into()),
            "cloud sync not reachable: timed out",
        ),
        (
            CloudSyncError::Unavailable { status: 503, code: "storage_unavailable".into() },
            "cloud sync unavailable (503 storage_unavailable)",
        ),
        (
            CloudSyncError::Rejected { status: 409, code: "bucket_name_taken".into() },
            "cloud sync refused the request (409 bucket_name_taken)",
        ),
        (
            CloudSyncError::BadResponse("`documents` is not a list".into()),
            "cloud sync sent an unexpected answer: `documents` is not a list",
        ),
        (
            CloudSyncError::Vault("store: locked".into()),
            "could not read the sign-in session: store: locked",
        ),
        (
            CloudSyncError::InvalidInput("not a document id".into()),
            "invalid cloud request: not a document id",
        ),
        (
            CloudSyncError::ObjectTooLarge { limit_bytes: 10_485_760 },
            "the document is over the cloud limit for one document (10485760 bytes)",
        ),
        (
            CloudSyncError::QuotaExceeded { limit_bytes: 1000, used_bytes: 990 },
            "the cloud storage limit is reached (990 of 1000 bytes used)",
        ),
        (
            CloudSyncError::UploadLinkExpired,
            "the upload link expired before the upload finished",
        ),
        (
            CloudSyncError::ObjectChanged,
            "the document changed in the cloud during the upload",
        ),
        (CloudSyncError::ObjectNotFound, "the document is not in the cloud"),
        (
            CloudSyncError::StorageTokenRejected,
            "cloud storage refused the storage token (401)",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
}
