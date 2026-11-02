//! The one error type of the cloud sync service.

use std::fmt;

/// Why a cloud call failed. The cases are told apart so the caller can act on each one: sign in,
/// refresh the session, retry later, or stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudSyncError {
    /// The vault holds no session: the user is not signed in.
    NotSignedIn,
    /// The session in the vault is past its `expires_at`. The UI refreshes it; this service does not.
    SessionExpired,
    /// The Worker answered 401: it does not accept the session token.
    Unauthorized,
    /// The Worker could not be reached, or did not answer in time.
    Network(String),
    /// The Worker, or Supabase or Google behind it, is unavailable (5xx, or Google's 429). `code`
    /// is the error name in the body, `unknown` when there is none.
    Unavailable { status: u16, code: String },
    /// The Worker or Google refused the request (4xx not named below).
    Rejected { status: u16, code: String },
    /// The answer does not follow the Worker contract.
    BadResponse(String),
    /// The vault could not be read.
    Vault(String),
    /// The caller gave something that cannot be a request, such as a document id that is not one.
    InvalidInput(String),
    /// The package is over the limit for one object (the Worker's 413 `object_too_large`).
    ObjectTooLarge { limit_bytes: u64 },
    /// The upload would take the user's cloud total over the limit (the Worker's 413 `quota_exceeded`).
    QuotaExceeded { limit_bytes: u64, used_bytes: u64 },
    /// Google refused the upload link as expired, and then refused the fresh one too.
    UploadLinkExpired,
    /// The object changed in the cloud after the upload link was signed (Google's 412).
    ObjectChanged,
    /// The cloud has no object for the document (Google's 404 on a read).
    ObjectNotFound,
    /// Google refused the storage token (401), and refused the new one too.
    StorageTokenRejected,
    /// The user cancelled the sync, so the call was not made. Only a run that can be cancelled
    /// (see `doc_sync::SyncGate::cancel`) ever answers with this.
    Cancelled,
}

impl fmt::Display for CloudSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSignedIn => write!(f, "not signed in"),
            Self::SessionExpired => write!(f, "sign-in session expired"),
            Self::Unauthorized => write!(f, "cloud sync refused the sign-in session (401)"),
            Self::Network(e) => write!(f, "cloud sync not reachable: {e}"),
            Self::Unavailable { status, code } => {
                write!(f, "cloud sync unavailable ({status} {code})")
            }
            Self::Rejected { status, code } => {
                write!(f, "cloud sync refused the request ({status} {code})")
            }
            Self::BadResponse(e) => write!(f, "cloud sync sent an unexpected answer: {e}"),
            Self::Vault(e) => write!(f, "could not read the sign-in session: {e}"),
            Self::InvalidInput(e) => write!(f, "invalid cloud request: {e}"),
            Self::ObjectTooLarge { limit_bytes } => {
                write!(f, "the document is over the cloud limit for one document ({limit_bytes} bytes)")
            }
            Self::QuotaExceeded { limit_bytes, used_bytes } => {
                write!(f, "the cloud storage limit is reached ({used_bytes} of {limit_bytes} bytes used)")
            }
            Self::UploadLinkExpired => write!(f, "the upload link expired before the upload finished"),
            Self::ObjectChanged => write!(f, "the document changed in the cloud during the upload"),
            Self::ObjectNotFound => write!(f, "the document is not in the cloud"),
            Self::StorageTokenRejected => write!(f, "cloud storage refused the storage token (401)"),
            Self::Cancelled => write!(f, "the sync was cancelled"),
        }
    }
}

impl std::error::Error for CloudSyncError {}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_error.rs"]
mod tests;
