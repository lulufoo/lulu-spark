//! The calls to the Worker: exchange the session for a storage token, list the cloud documents,
//! ask for an upload link, delete a document.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use reqwest::Method;
use serde_json::{json, Value};

use super::error::CloudSyncError;
use super::types::{
    check_document_id, parse_credentials, parse_documents, parse_upload_link, CloudDoc, Credentials,
    UploadLink,
};
use super::{STORAGE_URL, WORKER_URL};
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
    pub(super) storage_url: String,
    sessions: SessionSource,
    clock: Clock,
    cache: Mutex<Option<Cached>>,
    /// The one HTTP client of the service, built by the first call. Its connections are kept for
    /// the next call: a call that opens a new connection to Google or the Worker costs a TCP and
    /// a TLS handshake, and on a network that cuts some new connections, every handshake is a
    /// chance to stop the sync.
    http: OnceLock<Client>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

/// The error and its causes, e.g. `error sending request: connection refused`: reqwest keeps the
/// reason in the cause, not in its own text.
///
/// The URL is left out. The text goes to the log and to the UI, and the URL of an upload is a
/// signed link that lets anyone who holds it write the object for as long as it is valid.
pub(super) fn network(e: reqwest::Error) -> CloudSyncError {
    let e = e.without_url();
    let mut text = e.to_string();
    let mut cause = std::error::Error::source(&e);
    while let Some(c) = cause {
        text.push_str(": ");
        text.push_str(&c.to_string());
        cause = c.source();
    }
    CloudSyncError::Network(text)
}

// The steps of a sync that go over the network. A network error says which one it was in, so that
// the log and the UI can tell a link the Worker did not give from bytes Google did not take.
pub(super) const STEP_CREDENTIALS: &str = "credentials";
pub(super) const STEP_LIST: &str = "document list";
pub(super) const STEP_LINK: &str = "upload link";
pub(super) const STEP_UPLOAD: &str = "upload to storage";
pub(super) const STEP_DOWNLOAD: &str = "download from storage";
pub(super) const STEP_DELETE: &str = "delete";

/// `error` with `step` in front of its text when it is a network error, e.g.
/// `upload link: error sending request: ...`. Any other error is as it was.
pub(super) fn in_step(step: &str, error: CloudSyncError) -> CloudSyncError {
    match error {
        CloudSyncError::Network(text) => CloudSyncError::Network(format!("{step}: {text}")),
        other => other,
    }
}

/// [`network`] for `map_err`, with the step in front.
pub(super) fn network_in(step: &'static str) -> impl Fn(reqwest::Error) -> CloudSyncError {
    move |e| in_step(step, network(e))
}

/// The client every call goes through. It follows no redirects: neither the Worker nor Google
/// redirects, and the session token must not follow one. `REQUEST_TIMEOUT` is for a call that
/// sets none of its own; a transfer sets a longer one on its request.
fn build_http() -> Result<Client, CloudSyncError> {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(Policy::none())
        .build()
        .map_err(network)
}

/// The Worker's own limits come with numbers, so the caller can say which one was hit.
fn limit_error(code: &str, body: &Value) -> Option<CloudSyncError> {
    let number = |field: &str| body.get(field).and_then(Value::as_u64);
    match code {
        "object_too_large" => Some(CloudSyncError::ObjectTooLarge { limit_bytes: number("limitBytes")? }),
        "quota_exceeded" => Some(CloudSyncError::QuotaExceeded {
            limit_bytes: number("limitBytes")?,
            used_bytes: number("usedBytes")?,
        }),
        _ => None,
    }
}

fn failure(status: u16, body: &[u8]) -> CloudSyncError {
    let body: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let code = body
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    match status {
        401 => CloudSyncError::Unauthorized,
        413 => limit_error(&code, &body).unwrap_or(CloudSyncError::Rejected { status, code }),
        500..=599 => CloudSyncError::Unavailable { status, code },
        400..=499 => CloudSyncError::Rejected { status, code },
        _ => CloudSyncError::BadResponse(format!("unexpected status {status}")),
    }
}

impl CloudSync {
    pub fn new(base_url: &str, storage_url: &str, sessions: SessionSource, clock: Clock) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            storage_url: storage_url.trim_end_matches('/').to_string(),
            sessions,
            clock,
            cache: Mutex::new(None),
            http: OnceLock::new(),
        }
    }

    /// The client of the service, built on first use. Every request goes through it, so the
    /// connections it opened are used again. A request waits at most `REQUEST_TIMEOUT`, unless it
    /// sets its own.
    pub(super) fn http(&self) -> Result<&Client, CloudSyncError> {
        if let Some(client) = self.http.get() {
            return Ok(client);
        }
        // Two calls that start together may both build one: the first stored is kept.
        let built = build_http()?;
        Ok(self.http.get_or_init(|| built))
    }

    /// The deployed Worker, Google Cloud Storage, the session in the vault, the system clock.
    pub fn production() -> Self {
        Self::new(
            WORKER_URL,
            STORAGE_URL,
            Arc::new(vault::get_auth_session),
            Arc::new(unix_now),
        )
    }

    /// The session to call the Worker with. Never refreshed here.
    pub(super) fn session(&self) -> Result<AuthSession, CloudSyncError> {
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
        body: Option<&Value>,
        step: &'static str,
    ) -> Result<Vec<u8>, CloudSyncError> {
        let mut request = self
            .http()?
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(&session.access_token);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().map_err(network_in(step))?;
        let status = response.status();
        let body = response.bytes().map_err(network_in(step))?;
        if !status.is_success() {
            return Err(failure(status.as_u16(), &body));
        }
        Ok(body.to_vec())
    }

    fn call_json(
        &self,
        method: Method,
        path: &str,
        session: &AuthSession,
        body: Option<&Value>,
        step: &'static str,
    ) -> Result<Value, CloudSyncError> {
        serde_json::from_slice(&self.call(method, path, session, body, step)?)
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
        let body = self.call_json(Method::POST, "/credentials", &session, None, STEP_CREDENTIALS)?;
        let credentials = parse_credentials(&body)?;
        *cache = Some(Cached {
            user_id: session.user.id,
            credentials: credentials.clone(),
        });
        Ok(credentials)
    }

    /// Drops the stored token, so the next `credentials()` asks the Worker for a new one.
    pub(super) fn forget_credentials(&self) {
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// The `(document id, version)` pairs the cloud holds for the signed-in user.
    pub fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        let session = self.session()?;
        parse_documents(&self.call_json(Method::GET, "/documents", &session, None, STEP_LIST)?)
    }

    /// Asks the Worker for a link that writes this one object, `size` bytes at most, once.
    pub(super) fn upload_link(
        &self,
        document_id: &str,
        size: usize,
        version: i64,
    ) -> Result<UploadLink, CloudSyncError> {
        let session = self.session()?;
        let request = json!({ "documentId": document_id, "size": size, "version": version });
        let body = self.call_json(Method::POST, "/uploads", &session, Some(&request), STEP_LINK)?;
        parse_upload_link(&body)
    }

    /// Deletes the document's object. A document that is not in the cloud counts as deleted. The
    /// storage token only reads, so the Worker does the deleting.
    pub fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        check_document_id(document_id)?;
        let session = self.session()?;
        self.call(Method::DELETE, &format!("/documents/{document_id}"), &session, None, STEP_DELETE)
            .map(drop)
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_client.rs"]
mod tests;
