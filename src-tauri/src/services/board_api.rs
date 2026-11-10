//! One Board graph HTTP client for this process.
//!
//! Sketch reads and the startup warmup share it so the TLS session and the idle
//! connection can be reused. The warmup does not send the PAT.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::redirect::Policy;

use crate::services::board_sketch_log;

pub(crate) const GRAPH_URL: &str = "https://api.luluboard.app/graphs";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Long enough that opening a note after launch still hits the warmed connection.
const IDLE: Duration = Duration::from_secs(600);

pub fn spawn_warm() {
    std::thread::spawn(warm);
}

pub(crate) fn client() -> Option<&'static Client> {
    static CLIENT: OnceLock<Option<Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .pool_idle_timeout(IDLE)
                .redirect(Policy::none())
                .build()
                .ok()
        })
        .as_ref()
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
