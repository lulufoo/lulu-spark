//! Process-log lines for L4 decisions the HTTP layer cannot see: why a request was
//! refused and which slot a token was minted for. Business id `mcp_oauth`.
//!
//! Fields are ids, slots and reasons only. Never codes, PKCE values or tokens.

use serde_json::json;

use crate::services::mcp_oauth::Slot;
use crate::services::mcp_oauth_log::{clip, info, trace_for, warn};

pub(super) fn authorize_rejected(client_id: &str, redirect_uri: &str, reason: &str) {
    warn(
        "authorize.rejected",
        &trace_for(client_id),
        json!({
            "reason": reason,
            "client_id": clip(client_id),
            "redirect_uri": clip(redirect_uri),
        }),
    );
}

/// The registry is memory only, so a restart forgets clients: the request is still
/// accepted when the redirect URI has a valid shape. Worth seeing in the log.
pub(super) fn unknown_client(client_id: &str, redirect_uri: &str) {
    warn(
        "authorize.unknown_client",
        &trace_for(client_id),
        json!({
            "note": "client_id not registered; accepted by redirect uri shape",
            "client_id": clip(client_id),
            "redirect_uri": clip(redirect_uri),
        }),
    );
}

pub(super) fn token_rejected(client_id: &str, reason: &str) {
    warn(
        "token.rejected",
        &trace_for(client_id),
        json!({ "reason": reason, "client_id": clip(client_id) }),
    );
}

pub(super) fn token_issued(client_id: &str, slot: Slot) {
    info(
        "token.issued",
        &trace_for(client_id),
        json!({ "client_id": clip(client_id), "slot": slot.as_str() }),
    );
}
