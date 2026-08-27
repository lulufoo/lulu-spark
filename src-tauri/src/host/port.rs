use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
const RETRY_INTERVAL: Duration = Duration::from_millis(500);

fn localhost_addrs(port: u16) -> Vec<SocketAddr> {
    format!("localhost:{port}")
        .to_socket_addrs()
        .expect("resolve localhost")
        .collect()
}

pub fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let addrs = localhost_addrs(port);

    loop {
        if addrs
            .iter()
            .any(|addr| TcpStream::connect_timeout(addr, CONNECT_TIMEOUT).is_ok())
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(RETRY_INTERVAL);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpawnDecision {
    Skip,
    Spawn,
}

pub fn decide_spawn(port: u16, probe_timeout: Duration) -> SpawnDecision {
    if wait_for_port(port, probe_timeout) {
        SpawnDecision::Skip
    } else {
        SpawnDecision::Spawn
    }
}
