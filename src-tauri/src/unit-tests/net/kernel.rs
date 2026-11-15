//! Kernel routing, proven against a local observable proxy: a request that
//! goes through it leaves an absolute-form (or CONNECT) record behind; a
//! request that bypasses it leaves none.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use super::*;

use crate::net::spec::ClientSpec;

/// A local HTTP proxy that records every request line and answers 200.
pub(crate) struct ObservableProxy {
    pub(crate) port: u16,
    hits: Arc<Mutex<Vec<String>>>,
    server: Arc<tiny_http::Server>,
    join: Option<JoinHandle<()>>,
}

impl ObservableProxy {
    pub(crate) fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("proxy listen");
        let port = listener.local_addr().expect("proxy addr").port();
        let server = Arc::new(tiny_http::Server::from_listener(listener, None).expect("proxy server"));
        let hits = Arc::new(Mutex::new(Vec::new()));
        let server_thread = Arc::clone(&server);
        let hits_thread = Arc::clone(&hits);
        let join = std::thread::spawn(move || {
            for request in server_thread.incoming_requests() {
                let line = format!("{} {}", request.method(), request.url());
                hits_thread.lock().expect("proxy hits").push(line);
                let response =
                    tiny_http::Response::from_string("proxied-hit").with_status_code(tiny_http::StatusCode(200));
                let _ = request.respond(response);
            }
        });
        Self {
            port,
            hits,
            server,
            join: Some(join),
        }
    }

    fn env_value(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn endpoint(&self) -> crate::host::ProxyEndpoint {
        crate::host::ProxyEndpoint {
            host: "127.0.0.1".to_string(),
            port: Some(self.port),
        }
    }

    pub(crate) fn hits(&self) -> Vec<String> {
        self.hits.lock().expect("proxy hits").clone()
    }
}

impl Drop for ObservableProxy {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// A plain loopback origin server for direct-connection evidence.
struct LocalOrigin {
    port: u16,
    hits: Arc<Mutex<Vec<String>>>,
    server: Arc<tiny_http::Server>,
    join: Option<JoinHandle<()>>,
}

impl LocalOrigin {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("origin listen");
        let port = listener.local_addr().expect("origin addr").port();
        let server = Arc::new(tiny_http::Server::from_listener(listener, None).expect("origin server"));
        let hits = Arc::new(Mutex::new(Vec::new()));
        let server_thread = Arc::clone(&server);
        let hits_thread = Arc::clone(&hits);
        let join = std::thread::spawn(move || {
            for request in server_thread.incoming_requests() {
                hits_thread
                    .lock()
                    .expect("origin hits")
                    .push(format!("{} {}", request.method(), request.url()));
                let response =
                    tiny_http::Response::from_string("direct-hit").with_status_code(tiny_http::StatusCode(200));
                let _ = request.respond(response);
            }
        });
        Self {
            port,
            hits,
            server,
            join: Some(join),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn hits(&self) -> Vec<String> {
        self.hits.lock().expect("origin hits").clone()
    }
}

impl Drop for LocalOrigin {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn spec() -> ClientSpec {
    ClientSpec::default().with_request_timeout(Duration::from_secs(5))
}

#[test]
fn system_proxy_configured_and_on_hits_the_proxy_for_plain_http() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy = ObservableProxy::start();
    let kernel = kernel_with(true, snapshot(Some(("127.0.0.1", Some(proxy.port))), None, &[]));
    let client = kernel.external(&spec()).expect("external client");
    let response = client
        .get("http://spark.test/echo")
        .send()
        .expect("request must travel through the proxy");
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().expect("body"), "proxied-hit");
    let hits = proxy.hits();
    assert_eq!(
        hits,
        vec!["GET http://spark.test/echo".to_string()],
        "absolute-form request line is the proxy-hit evidence"
    );
}

#[test]
fn env_proxy_wins_over_system_and_survives_toggle_off() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy_env = ObservableProxy::start();
    let proxy_system = ObservableProxy::start();
    set_env("HTTP_PROXY", &proxy_env.env_value());
    let kernel = kernel_with(
        true,
        snapshot(
            Some(("127.0.0.1", Some(proxy_system.port))),
            Some(("127.0.0.1", Some(proxy_system.port))),
            &[],
        ),
    );
    let client = kernel.external(&spec()).expect("external client");
    client.get("http://spark.test/first").send().expect("first request");
    assert_eq!(proxy_env.hits().len(), 1, "env source must win");
    assert!(proxy_system.hits().is_empty(), "system source must lose to env");

    // Toggle off: the env source keeps working, the system one stays silent.
    let toggle: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(|| false);
    assert!(
        kernel.install(
            toggle,
            fake_source(snapshot(
                Some(("127.0.0.1", Some(proxy_system.port))),
                Some(("127.0.0.1", Some(proxy_system.port))),
                &[],
            ))
        ),
        "toggling off must change the plan"
    );
    client.get("http://spark.test/second").send().expect("second request");
    assert_eq!(proxy_env.hits().len(), 2, "env proxy keeps working with the toggle off");
    assert!(proxy_system.hits().is_empty());
}

#[test]
fn exception_list_sends_matching_hosts_direct() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy = ObservableProxy::start();

    // System exceptions, toggle on.
    let kernel = kernel_with(
        true,
        snapshot(Some(("127.0.0.1", Some(proxy.port))), None, &["spark.test"]),
    );
    let client = kernel.external(&spec()).expect("external client");
    assert!(
        client.get("http://spark.test/echo").send().is_err(),
        "excluded host goes direct and spark.test does not resolve"
    );
    assert!(proxy.hits().is_empty(), "excluded host must bypass the proxy");

    // Env NO_PROXY, toggle off.
    set_env("HTTP_PROXY", &proxy.env_value());
    set_env("NO_PROXY", "spark.test");
    let kernel = kernel_with(false, snapshot(None, None, &[]));
    let client = kernel.external(&spec()).expect("external client");
    assert!(client.get("http://spark.test/echo").send().is_err());
    assert!(proxy.hits().is_empty(), "NO_PROXY must bypass the proxy");
}

#[test]
fn loopback_stays_direct_even_with_proxy_configured() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy = ObservableProxy::start();
    let origin = LocalOrigin::start();
    set_env("HTTP_PROXY", &proxy.env_value());
    let kernel = kernel_with(true, snapshot(Some(("127.0.0.1", Some(proxy.port))), None, &[]));
    let client = kernel.loopback(Duration::from_secs(3)).expect("loopback client");
    let response = client.get(origin.url()).send().expect("direct request");
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().expect("body"), "direct-hit");
    assert_eq!(origin.hits().len(), 1);
    assert!(proxy.hits().is_empty(), "loopback must never enter the proxy");
}

#[test]
fn hot_switch_moves_new_acquisition_to_the_new_generation_and_keeps_old_clients() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy = ObservableProxy::start();
    let origin = LocalOrigin::start();
    let kernel = kernel_with(false, snapshot(Some(("127.0.0.1", Some(proxy.port))), None, &[]));
    let old = kernel.external(&spec()).expect("client before switch");
    let response = old.get(origin.url()).send().expect("direct request before switch");
    assert_eq!(response.status().as_u16(), 200);

    let toggle: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(|| true);
    assert!(kernel.install(toggle, fake_source(snapshot(Some(("127.0.0.1", Some(proxy.port))), None, &[]))));
    let new = kernel.external(&spec()).expect("client after switch");
    assert!(
        !Arc::ptr_eq(&old, &new),
        "a plan change must produce a new client for the same spec"
    );
    new.get("http://spark.test/after-switch").send().expect("proxied after switch");
    assert_eq!(proxy.hits(), vec!["GET http://spark.test/after-switch".to_string()]);

    // The old client keeps working: in-flight and pooled callers are safe.
    let response = old.get(origin.url()).send().expect("old client after switch");
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(origin.hits().len(), 2);
}

#[test]
fn same_spec_shares_one_client_within_a_generation() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let kernel = kernel_with(false, snapshot(None, None, &[]));
    let first = kernel.external(&spec()).expect("first");
    let second = kernel.external(&spec()).expect("second");
    assert!(Arc::ptr_eq(&first, &second), "same spec must reuse the client");
}

#[test]
fn connect_evidence_reaches_the_proxy_for_https_targets() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy = ObservableProxy::start();
    let kernel = kernel_with(true, snapshot(None, Some(("127.0.0.1", Some(proxy.port))), &[]));
    let client = kernel.external(&spec()).expect("external client");
    // TLS to a fake target fails, but the CONNECT record is the evidence.
    let _ = client.get("https://spark.test/resource").send();
    let hits = proxy.hits();
    assert_eq!(hits.len(), 1, "exactly one CONNECT: {hits:?}");
    assert!(hits[0].starts_with("CONNECT spark.test:443"), "CONNECT line: {}", hits[0]);
}

#[test]
fn snapshot_json_reports_generation_without_credentials() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    set_env("HTTP_PROXY", "http://user:pass@proxy.corp:3128");
    let kernel = kernel_with(false, snapshot(None, None, &[]));
    let snapshot = kernel.snapshot_json();
    let text = snapshot.to_string();
    assert!(text.contains("\"generation\""));
    assert!(text.contains("proxy.corp:3128"));
    assert!(!text.contains("pass"), "snapshot must not leak credentials: {text}");
}
