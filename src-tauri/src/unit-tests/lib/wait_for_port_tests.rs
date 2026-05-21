use super::*;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

fn listen_on_ephemeral() -> (TcpListener, u16) {
    let listener = TcpListener::bind("localhost:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    (listener, port)
}

#[test]
fn returns_true_when_port_already_listening() {
    let (_listener, port) = listen_on_ephemeral();
    let start = Instant::now();
    assert!(wait_for_port(port, Duration::from_secs(2)));
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn returns_true_when_port_listens_after_delay() {
    let port = {
        let listener = TcpListener::bind("localhost:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);
        port
    };
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        let _listener = TcpListener::bind(format!("localhost:{port}")).expect("bind delayed");
        thread::sleep(Duration::from_secs(5));
    });
    let start = Instant::now();
    assert!(wait_for_port(port, Duration::from_secs(2)));
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(500));
    assert!(elapsed < Duration::from_secs(2));
}

#[test]
fn zero_timeout_on_closed_port_returns_false_immediately() {
    let port = {
        let listener = TcpListener::bind("localhost:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);
        port
    };
    let start = Instant::now();
    assert!(!wait_for_port(port, Duration::from_millis(0)));
    assert!(start.elapsed() < Duration::from_millis(200));
}

#[test]
fn zero_timeout_on_open_port_returns_true() {
    let (_listener, port) = listen_on_ephemeral();
    assert!(wait_for_port(port, Duration::from_millis(0)));
}

#[test]
fn returns_false_after_timeout_when_never_listening() {
    let port = {
        let listener = TcpListener::bind("localhost:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);
        port
    };
    let start = Instant::now();
    assert!(!wait_for_port(port, Duration::from_millis(800)));
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(600));
    assert!(elapsed <= Duration::from_millis(1200));
}

#[test]
fn port_65535_closed_returns_false_without_panic() {
    assert!(!wait_for_port(65535, Duration::from_secs(1)));
}
