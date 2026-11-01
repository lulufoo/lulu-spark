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
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
}
