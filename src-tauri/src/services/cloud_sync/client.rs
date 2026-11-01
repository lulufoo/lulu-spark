//! The calls to the Worker: exchange the session for a storage token, list the cloud documents.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use reqwest::Method;
use serde_json::Value;

use super::error::CloudSyncError;
use super::types::{parse_credentials, parse_documents, CloudDoc, Credentials};
use super::WORKER_URL;
use crate::config::vault::{self, AuthSession, SecretError};

/// Where the signed-in session comes from. The app reads the vault.
pub type SessionSource = Arc<dyn Fn() -> Result<Option<AuthSession>, SecretError> + Send + Sync>;
/// Seconds since the Unix epoch.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// The first call for a new user waits while the Worker creates the bucket.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// A cached token is used only while it has more than this many seconds left. The Worker issues
/// tokens that last at least 30 minutes.
const REFRESH_MARGIN_SECS: i64 = 300;

struct Cached {
    user_id: String,
    credentials: Credentials,
}

/// Blocking calls: run them on a worker thread, not inside an async task.
pub struct CloudSync {
    base_url: String,
    sessions: SessionSource,
    clock: Clock,
    cache: Mutex<Option<Cached>>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

/// The error and its causes, e.g. `error sending request: connection refused`: reqwest keeps the
/// reason in the cause, not in its own text.
fn network(e: reqwest::Error) -> CloudSyncError {
    let mut text = e.to_string();
    let mut cause = std::error::Error::source(&e);
    while let Some(c) = cause {
        text.push_str(": ");
        text.push_str(&c.to_string());
        cause = c.source();
    }
    CloudSyncError::Network(text)
}

fn failure(status: u16, body: &[u8]) -> CloudSyncError {
    let code = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());
    match status {
        401 => CloudSyncError::Unauthorized,
        500..=599 => CloudSyncError::Unavailable { status, code },
        400..=499 => CloudSyncError::Rejected { status, code },
        _ => CloudSyncError::BadResponse(format!("unexpected status {status}")),
    }
}

impl CloudSync {
    pub fn new(base_url: &str, sessions: SessionSource, clock: Clock) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            sessions,
            clock,
            cache: Mutex::new(None),
        }
    }

    /// The deployed Worker, the session in the vault, the system clock.
    pub fn production() -> Self {
        Self::new(
            WORKER_URL,
            Arc::new(vault::get_auth_session),
            Arc::new(unix_now),
        )
    }

    /// The session to call the Worker with. Never refreshed here.
    fn session(&self) -> Result<AuthSession, CloudSyncError> {
        let session = (self.sessions)()
            .map_err(|e| CloudSyncError::Vault(e.to_string()))?
            .ok_or(CloudSyncError::NotSignedIn)?;
        if session.expires_at <= (self.clock)() {
            return Err(CloudSyncError::SessionExpired);
        }
        Ok(session)
    }

    fn call(
        &self,
        method: Method,
        path: &str,
        session: &AuthSession,
    ) -> Result<Value, CloudSyncError> {
        // No redirects: the Worker never redirects, and the session token must not follow one.
        let http = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .build()
            .map_err(network)?;
        let response = http
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(&session.access_token)
            .send()
            .map_err(network)?;
        let status = response.status();
        let body = response.bytes().map_err(network)?;
        if !status.is_success() {
            return Err(failure(status.as_u16(), &body));
        }
        serde_json::from_slice(&body)
            .map_err(|e| CloudSyncError::BadResponse(format!("body is not JSON: {e}")))
    }

    /// A read-only token for the user's bucket. The last one is kept and reused until shortly
    /// before it expires; then the Worker is asked again.
    pub fn credentials(&self) -> Result<Credentials, CloudSyncError> {
        let session = self.session()?;
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(kept) = cache.as_ref() {
            let fresh = kept.credentials.expires_at - REFRESH_MARGIN_SECS > (self.clock)();
            if fresh && kept.user_id == session.user.id {
                return Ok(kept.credentials.clone());
            }
        }
        let credentials = parse_credentials(&self.call(Method::POST, "/credentials", &session)?)?;
        *cache = Some(Cached {
            user_id: session.user.id,
            credentials: credentials.clone(),
        });
        Ok(credentials)
    }

    /// The `(document id, version)` pairs the cloud holds for the signed-in user.
    pub fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        let session = self.session()?;
        parse_documents(&self.call(Method::GET, "/documents", &session)?)
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_client.rs"]
mod tests;
