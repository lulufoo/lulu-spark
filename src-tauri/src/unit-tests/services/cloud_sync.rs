//! Fixtures shared by the cloud sync tests: a mock Worker, a movable clock, a switchable session.

use std::net::TcpListener;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde_json::{json, Value};
use tiny_http::{Header, Response, Server};

use super::{shared, Clock, CloudSync, SessionSource, STORAGE_URL, WORKER_URL};
use crate::config::vault::{AuthSession, AuthUser, SecretError};

pub(super) const NOW: i64 = 1_800_000_000;
pub(super) const JWT: &str = "jwt-secret-1";
pub(super) const USER: &str = "user-1";
pub(super) const DOC_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const DOC_B: &str = "0123456789abcdef0123456789abcdef";

/// What the Worker saw of one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Seen {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) authorization: Option<String>,
}

pub(super) fn seen(method: &str, path: &str, token: &str) -> Seen {
    Seen {
        method: method.into(),
        path: path.into(),
        authorization: Some(format!("Bearer {token}")),
    }
}

/// Everything the Worker saw of one request: header names are lower case. `peer` is the address
/// the request came from: the port is new for every TCP connection, so requests that share a
/// connection share a `peer`.
#[derive(Debug, Clone)]
pub(super) struct Recorded {
    pub(super) peer: String,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl Recorded {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    pub(super) fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("the body is JSON")
    }
}

/// One scripted answer. `{mock}` in a body stands for the address of the mock itself, which is not
/// known before it starts: that is how a link can point back at it.
#[derive(Debug, Clone)]
pub(super) struct Reply {
    status: u16,
    body: Vec<u8>,
    headers: Vec<(&'static str, String)>,
}

impl Reply {
    pub(super) fn json(status: u16, body: Value) -> Self {
        Self::raw(status, &body.to_string())
    }

    pub(super) fn raw(status: u16, body: &str) -> Self {
        Self::bytes(status, body.as_bytes())
    }

    pub(super) fn bytes(status: u16, body: &[u8]) -> Self {
        Self {
            status,
            body: body.to_vec(),
            headers: Vec::new(),
        }
    }

    pub(super) fn redirect(location: &str) -> Self {
        let mut reply = Self::raw(302, "");
        reply.headers.push(("Location", location.into()));
        reply
    }
}

pub(super) fn creds_reply(access_token: &str, expires_at: i64) -> Reply {
    Reply::json(
        200,
        json!({ "bucket": "spark-abc", "accessToken": access_token, "expiresAt": expires_at }),
    )
}

/// What `POST /uploads` answers: a PUT to `path` on the mock, with the headers Google will check.
pub(super) fn link_reply(path: &str) -> Reply {
    Reply::json(
        200,
        json!({
            "method": "PUT",
            "url": format!("{{mock}}{path}?X-Goog-Signature=abc"),
            "headers": {
                "x-goog-content-length-range": "0,1000000",
                "x-goog-if-generation-match": "0",
                "x-goog-meta-version": "3",
            },
            "expiresAt": NOW + 600,
        }),
    )
}

/// Google's XML answer to an expired link.
pub(super) fn expired_reply() -> Reply {
    xml_error(400, "ExpiredToken")
}

pub(super) fn xml_error(status: u16, code: &str) -> Reply {
    Reply::raw(
        status,
        &format!("<?xml version='1.0' encoding='UTF-8'?><Error><Code>{code}</Code><Message>m</Message></Error>"),
    )
}

pub(super) fn list_reply(documents: &[(&str, i64)]) -> Reply {
    let documents: Vec<Value> = documents
        .iter()
        .map(|(id, version)| json!({ "documentId": id, "version": version }))
        .collect();
    Reply::json(200, json!({ "documents": documents }))
}

/// A Worker on a free local port that answers the scripted replies in order, and with a 500 once
/// they run out.
pub(super) struct MockWorker {
    pub(super) url: String,
    recorded: Arc<Mutex<Vec<Recorded>>>,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl MockWorker {
    pub(super) fn start(replies: Vec<Reply>) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").expect("bind mock worker"));
        let url = format!("http://{}", server.server_addr().to_ip().expect("ip address"));
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let thread = {
            let (server, recorded, own_url) = (server.clone(), recorded.clone(), url.clone());
            let mut replies = replies.into_iter();
            std::thread::spawn(move || {
                for mut request in server.incoming_requests() {
                    let mut body = Vec::new();
                    let _ = request.as_reader().read_to_end(&mut body);
                    recorded.lock().unwrap().push(Recorded {
                        peer: request.remote_addr().map(|a| a.to_string()).unwrap_or_default(),
                        method: request.method().as_str().to_string(),
                        path: request.url().to_string(),
                        headers: request
                            .headers()
                            .iter()
                            .map(|h| (h.field.as_str().as_str().to_ascii_lowercase(), h.value.to_string()))
                            .collect(),
                        body,
                    });
                    let reply = replies
                        .next()
                        .unwrap_or_else(|| Reply::json(500, json!({ "error": "script_ran_out" })));
                    let text = String::from_utf8_lossy(&reply.body);
                    let body = if text.contains("{mock}") {
                        text.replace("{mock}", &own_url).into_bytes()
                    } else {
                        reply.body
                    };
                    let mut response = Response::from_data(body)
                        .with_status_code(reply.status)
                        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
                    for (name, value) in reply.headers {
                        response = response.with_header(Header::from_bytes(name, value).unwrap());
                    }
                    let _ = request.respond(response);
                }
            })
        };
        Self {
            url,
            recorded,
            server,
            thread: Some(thread),
        }
    }

    pub(super) fn recorded(&self) -> Vec<Recorded> {
        self.recorded.lock().unwrap().clone()
    }

    /// How many TCP connections the requests so far came over.
    pub(super) fn connections(&self) -> usize {
        let peers: std::collections::HashSet<String> =
            self.recorded().into_iter().map(|r| r.peer).collect();
        peers.len()
    }

    /// Method, path and `Authorization` of each request.
    pub(super) fn seen(&self) -> Vec<Seen> {
        self.recorded()
            .into_iter()
            .map(|r| Seen {
                authorization: r.header("authorization").map(str::to_string),
                method: r.method,
                path: r.path,
            })
            .collect()
    }
}

impl Drop for MockWorker {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A server that answers once with a body it promises 1000 bytes of, sends 5, and hangs up: the
/// answer is cut. It takes one connection.
pub(super) fn cutting_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    std::thread::spawn(move || {
        if let Some(Ok(mut stream)) = listener.incoming().next() {
            let mut request = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut request);
            let _ = std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\nshort",
            );
        }
    });
    url
}

/// A local address nobody listens on.
pub(super) fn dead_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    format!("http://{}", listener.local_addr().expect("address"))
}

pub(super) fn session(user: &str, token: &str, expires_at: i64) -> AuthSession {
    AuthSession {
        access_token: token.into(),
        refresh_token: "refresh-secret".into(),
        expires_at,
        user: AuthUser {
            id: user.into(),
            email: None,
            name: None,
            avatar: None,
            provider: None,
        },
    }
}

/// A mock Worker, a session the test can switch, and a clock the test can move. Starts at `NOW`,
/// signed in as `USER` with `JWT`, for one more hour.
pub(super) struct Rig {
    pub(super) worker: MockWorker,
    pub(super) sync: CloudSync,
    now: Arc<AtomicI64>,
    vault: Arc<Mutex<Result<Option<AuthSession>, String>>>,
}

impl Rig {
    pub(super) fn new(replies: Vec<Reply>) -> Self {
        let worker = MockWorker::start(replies);
        let url = worker.url.clone();
        Self::with_url(worker, &url)
    }

    /// `worker` is kept alive, but the service talks to `url`.
    pub(super) fn with_url(worker: MockWorker, url: &str) -> Self {
        let now = Arc::new(AtomicI64::new(NOW));
        let vault = Arc::new(Mutex::new(Ok(Some(session(USER, JWT, NOW + 3600)))));
        let sessions: SessionSource = {
            let vault = vault.clone();
            Arc::new(move || {
                vault
                    .lock()
                    .unwrap()
                    .clone()
                    .map_err(SecretError::Store)
            })
        };
        let clock: Clock = {
            let now = now.clone();
            Arc::new(move || now.load(Ordering::SeqCst))
        };
        let kernel = crate::net::tests::pinned_kernel();
        // Short timeouts: the stand-in proxy cannot complete a TLS tunnel,
        // so the transfer fails fast; the evidence is the CONNECT record.
        let spec = crate::net::ClientSpec::default()
            .with_connect_timeout(Duration::from_secs(2))
            .with_request_timeout(Duration::from_secs(3))
            .with_redirect(crate::net::RedirectSpec::None);
        let http_source: crate::services::cloud_sync::client::HttpSource = {
            let kernel = Arc::clone(&kernel);
            Arc::new(move || {
                kernel.external(&spec).map_err(|e| {
                    crate::services::cloud_sync::error::CloudSyncError::Network(e.to_string())
                })
            })
        };
        Self {
            sync: CloudSync::new(url, url, sessions, clock).with_http_source(http_source),
            worker,
            now,
            vault,
        }
    }

    pub(super) fn at(&self, now: i64) {
        self.now.store(now, Ordering::SeqCst);
    }

    pub(super) fn sign_in(&self, user: &str, token: &str, expires_at: i64) {
        *self.vault.lock().unwrap() = Ok(Some(session(user, token, expires_at)));
    }

    pub(super) fn sign_out(&self) {
        *self.vault.lock().unwrap() = Ok(None);
    }

    pub(super) fn vault_fails(&self, why: &str) {
        *self.vault.lock().unwrap() = Err(why.to_string());
    }
}

#[test]
fn the_worker_address_is_https_without_a_trailing_slash() {
    assert_eq!(WORKER_URL, "https://cloud-sync.luluspark.app");
}

#[test]
fn the_app_has_one_shared_service() {
    assert!(std::ptr::eq(shared(), shared()));
}

// --- unified network kernel (LW-812): the paths leave proxy evidence ---

#[test]
fn worker_calls_travel_through_the_kernel_client() {
    let _lock = crate::net::tests::lock_env();
    let proxy = crate::net::tests::kernel::ObservableProxy::start();
    let outcome;
    {
        let _clean = crate::net::tests::EnvCleaner;
        crate::net::tests::set_env("HTTP_PROXY", &proxy.env_value());
        crate::net::refresh_now();
        let session = AuthSession {
            access_token: JWT.into(),
            refresh_token: "refresh".into(),
            expires_at: NOW + 600,
            user: AuthUser {
                id: USER.into(),
                email: None,
                name: None,
                avatar: None,
                provider: None,
            },
        };
        let sync = CloudSync::new(
            "http://cloud-sync.test",
            STORAGE_URL,
            Arc::new(move || Ok(Some(session.clone()))),
            Arc::new(|| NOW),
        );
        outcome = sync.credentials();
    }
    crate::net::refresh_now();
    assert!(outcome.is_err(), "the proxy stand-in does not speak worker JSON: {outcome:?}");
    let hits = proxy.hits();
    assert_eq!(hits.len(), 1, "exactly one proxied request: {hits:?}");
    assert_eq!(hits[0], "POST http://cloud-sync.test/credentials");
}

#[test]
fn gcs_upload_travels_through_the_kernel_client_while_the_worker_stays_direct() {
    let _lock = crate::net::tests::lock_env();
    let proxy = crate::net::tests::kernel::ObservableProxy::start();
    let upload;
    let seen;
    {
        let _clean = crate::net::tests::EnvCleaner;
        crate::net::tests::set_env("HTTPS_PROXY", &proxy.env_value());
        crate::net::refresh_now();
        let link = Reply::json(
            200,
            json!({
                "method": "PUT",
                "url": "https://storage.googleapis.com/upload/storage/v1/b/spark-abc/o/upload?X-Goog-Signature=abc",
                "headers": { "x-goog-content-length-range": "0,1000000" },
                "expiresAt": NOW + 600,
            }),
        );
        let worker = MockWorker::start(vec![link]);
        let kernel = crate::net::tests::pinned_kernel();
        // Short timeouts: the stand-in proxy cannot complete a TLS tunnel,
        // so the transfer fails fast; the evidence is the CONNECT record.
        let spec = crate::net::ClientSpec::default()
            .with_connect_timeout(Duration::from_secs(2))
            .with_request_timeout(Duration::from_secs(3))
            .with_redirect(crate::net::RedirectSpec::None);
        let http_source: crate::services::cloud_sync::client::HttpSource = {
            let kernel = Arc::clone(&kernel);
            Arc::new(move || {
                kernel.external(&spec).map_err(|e| {
                    crate::services::cloud_sync::error::CloudSyncError::Network(e.to_string())
                })
            })
        };
        let sessions: SessionSource = Arc::new(|| Ok(Some(session(USER, JWT, NOW + 3600))));
        let sync = CloudSync::new(&worker.url, STORAGE_URL, sessions, Arc::new(|| NOW))
            .with_http_source(http_source);
        upload = sync.upload(DOC_A, b"package", 3);
        seen = worker.seen();
    }
    crate::net::refresh_now();
    assert!(
        upload.is_err(),
        "the stand-in tunnel cannot complete TLS: {upload:?}"
    );
    // The Worker hop stayed on the loopback mock; only the GCS hop went out.
    assert_eq!(seen.len(), 1, "only the upload link hits the mock worker");
    let hits = proxy.hits();
    assert_eq!(hits.len(), 1, "exactly one proxied request: {hits:?}");
    assert!(
        hits[0].starts_with("CONNECT storage.googleapis.com:443"),
        "GCS evidence is the CONNECT record: {}",
        hits[0]
    );
}
