//! Fixtures shared by the cloud sync tests: a mock Worker, a movable clock, a switchable session.

use std::net::TcpListener;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde_json::{json, Value};
use tiny_http::{Header, Response, Server};

use super::{shared, Clock, CloudSync, SessionSource, WORKER_URL};
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

/// Everything the Worker saw of one request: header names are lower case.
#[derive(Debug, Clone)]
pub(super) struct Recorded {
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
        Self {
            sync: CloudSync::new(url, url, sessions, clock),
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
    assert!(WORKER_URL.starts_with("https://"));
    assert!(!WORKER_URL.ends_with('/'));
}

#[test]
fn the_app_has_one_shared_service() {
    assert!(std::ptr::eq(shared(), shared()));
}
