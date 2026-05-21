use super::*;
use std::net::TcpListener;
use std::time::Duration;


#[test]
fn decide_spawn_skips_when_port_is_listening() {
    let listener = TcpListener::bind("localhost:0").expect("bind");
    let port = listener.local_addr().expect("local addr").port();
    assert_eq!(
        decide_spawn(port, Duration::from_millis(200)),
        SpawnDecision::Skip
    );
}

#[test]
fn decide_spawn_spawns_when_port_is_closed() {
    let listener = TcpListener::bind("localhost:0").expect("bind");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    assert_eq!(
        decide_spawn(port, Duration::from_millis(0)),
        SpawnDecision::Spawn
    );
}
