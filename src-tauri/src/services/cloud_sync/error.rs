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
    /// The Worker, or Supabase or Google behind it, is unavailable (5xx). `code` is the `error`
    /// field of the body, `unknown` when there is none.
    Unavailable { status: u16, code: String },
    /// The Worker refused the request (4xx other than 401).
    Rejected { status: u16, code: String },
    /// The answer does not follow the Worker contract.
    BadResponse(String),
    /// The vault could not be read.
    Vault(String),
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
        }
    }
}

impl std::error::Error for CloudSyncError {}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_error.rs"]
mod tests;
