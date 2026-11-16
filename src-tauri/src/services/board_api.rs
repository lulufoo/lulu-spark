//! Board HTTP access through the unified network kernel (LW-812).
//!
//! The kernel keeps one client per plan generation for this spec, so the TLS
//! session and the idle connection are reused across the startup warmup,
//! sketch reads, and background refreshes. The warmup does not send the PAT.

use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

use crate::net::{self, ClientSpec, RedirectSpec};
use crate::services::board_sketch_log;

pub(crate) const GRAPH_URL: &str = "https://api.luluboard.app/graphs";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Long enough that opening a note after launch still hits the warmed connection.
const IDLE: Duration = Duration::from_secs(600);

fn spec() -> ClientSpec {
    ClientSpec::default()
        .with_request_timeout(REQUEST_TIMEOUT)
        .with_redirect(RedirectSpec::None)
        .with_pool_idle_timeout(IDLE)
}

pub fn spawn_warm() {
    std::thread::spawn(warm);
}

pub(crate) fn client() -> Option<Arc<Client>> {
    net::external(&spec()).ok()
}

fn warm() {
    let started = Instant::now();
    let Some(http) = client() else {
        board_sketch_log::warmup("client", started.elapsed(), None, None);
        return;
    };
    match http
        .get(GRAPH_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16();
            let bytes = response.bytes().map(|b| b.len()).ok();
            board_sketch_log::warmup("http", started.elapsed(), Some(status), bytes);
        }
        Err(_) => board_sketch_log::warmup("network", started.elapsed(), None, None),
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/board_api.rs"]
mod tests;
