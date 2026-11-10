//! Process-log lines for Board Sketch reads (`app_log`, business `board_sketch`).
//!
//! Log timings a person needs to see why a preview was slow. Never log the PAT,
//! the Sketch HTML, or the page URL.

use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::services::app_log::{self, Level, Side};

pub const BUSINESS: &str = "board_sketch";
pub const EVENT_READ: &str = "sketch.read";

pub fn read(
    status: &str,
    diagram_id: &str,
    total: Duration,
    vault: Option<Duration>,
    fetch: Option<Duration>,
    bytes: Option<usize>,
) {
    let mut params = Map::new();
    params.insert("status".into(), json!(status));
    params.insert("ms".into(), json!(as_ms(total)));
    if !diagram_id.is_empty() {
        params.insert("diagram_id".into(), json!(diagram_id));
    }
    if let Some(d) = vault {
        params.insert("vault_ms".into(), json!(as_ms(d)));
    }
    if let Some(d) = fetch {
        params.insert("fetch_ms".into(), json!(as_ms(d)));
    }
    if let Some(n) = bytes {
        params.insert("bytes".into(), json!(n));
    }
    let level = match status {
        "sketch" | "not_sketch" | "not_board" => Level::Info,
        _ => Level::Warn,
    };
    app_log::log(BUSINESS, EVENT_READ, None, Some(Value::Object(params)), Side::Host, level);
}

fn as_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "../unit-tests/services/board_sketch_log.rs"]
mod tests;
