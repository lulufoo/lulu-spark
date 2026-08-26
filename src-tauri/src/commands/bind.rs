//! Local bind issue and session-read commands. Host-only; not mounted on Gateway.

use serde_json::{json, Value};
use tauri::State;

use crate::services::bind::{bind_session_state, BindError};
use crate::services::gateway::GatewayState;
use crate::services::lan_ip::current_lan_ipv4;

pub fn bind_error_to_command_error(err: BindError) -> String {
    format!("{err:?}")
}

pub fn issue_bind_with(gateway: &GatewayState) -> Result<Value, String> {
    let ip = current_lan_ipv4().ok_or_else(|| "no_lan".to_string())?;
    let listen = gateway.current().ok_or_else(|| "no_gateway".to_string())?;
    let tls_fingerprint = listen.tls_fingerprint;
    let payload = crate::services::bind::create_bind_payload(ip, listen.port, &tls_fingerprint)
        .map_err(bind_error_to_command_error)?;
    Ok(json!({
        "ip": ip.to_string(),
        "port": listen.port,
        "temp_pub": payload.temp_pub,
        "tls_fingerprint": tls_fingerprint,
        "exp": payload.exp,
        "sig": payload.sig,
    }))
}

#[tauri::command]
pub fn issue_bind(gateway: State<'_, GatewayState>) -> Result<Value, String> {
    issue_bind_with(&gateway)
}

#[tauri::command]
pub fn read_bind_session() -> String {
    format!("{:?}", bind_session_state())
}

#[cfg(test)]
#[path = "../unit-tests/commands/bind.rs"]
mod tests;
