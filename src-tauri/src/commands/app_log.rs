//! L1/L3 thin command: UI hops write the process execution log.
//! Host stamps process session, seq, pid, timestamp, and `side=ui`.

use serde_json::Value;

use crate::services::app_log::{self, Level, Side};

#[tauri::command]
pub fn log_app_event(
    business: String,
    event: String,
    trace_id: Option<String>,
    params: Option<Value>,
    level: Option<String>,
) -> Result<(), String> {
    if !app_log::token_ok(&business) || !app_log::token_ok(&event) {
        return Err("invalid app log business or event".into());
    }
    let level = level
        .as_deref()
        .and_then(Level::parse)
        .unwrap_or(Level::Info);
    app_log::log(
        &business,
        &event,
        trace_id.as_deref(),
        params,
        Side::Ui,
        level,
    );
    Ok(())
}

#[cfg(test)]
#[path = "../unit-tests/commands/app_log.rs"]
mod tests;
