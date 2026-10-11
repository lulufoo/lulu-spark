//! Who is calling the Host MCP server, for log lines only.
//!
//! Every request line carries the slot it hit, the client's `User-Agent`, and a
//! fingerprint of the Bearer it sent. A failed authentication also says why.
//! None of this changes what the gate answers: a 401 stays a uniform 401.
//!
//! The fingerprint is the first 8 hex characters of SHA-256 over the Bearer. A
//! ticket is 32 random bytes, so the hash cannot be turned back into it. No
//! prefix, suffix or plaintext of the Bearer is ever logged.

use axum::http::{header, HeaderMap};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::listen::bearer_secret;
use crate::services::mcp_oauth::{ledger_record, LedgerRecord, OAuthError, Slot, TicketState};

const FINGERPRINT_LEN: usize = 8;
const USER_AGENT_MAX: usize = 80;

/// First 8 hex characters of SHA-256 over `secret`.
pub(super) fn bearer_fingerprint(secret: &str) -> String {
    Sha256::digest(secret.as_bytes())
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(FINGERPRINT_LEN)
        .map(|nibble| char::from_digit(u32::from(nibble), 16).unwrap_or('0'))
        .collect()
}

/// `value` without control characters, cut to `max` characters. `None` when
/// nothing printable is left. Client-supplied text must never add log lines.
pub(super) fn clean_label(value: &str, max: usize) -> Option<String> {
    let cleaned: String = value.chars().filter(|c| !c.is_control()).take(max).collect();
    let cleaned = cleaned.trim();
    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

/// What one request says about its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Caller {
    slot: String,
    user_agent: Option<String>,
    bearer_fp: Option<String>,
}

impl Caller {
    pub(super) fn from_headers(slot: &str, headers: &HeaderMap) -> Self {
        Self {
            slot: slot.to_string(),
            user_agent: headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| clean_label(value, USER_AGENT_MAX)),
            bearer_fp: bearer_secret(headers).map(bearer_fingerprint),
        }
    }

    pub(super) fn add_to(&self, object: &mut Map<String, Value>) {
        object.insert("slot".into(), Value::String(self.slot.clone()));
        if let Some(user_agent) = &self.user_agent {
            object.insert("user_agent".into(), Value::String(user_agent.clone()));
        }
        if let Some(bearer_fp) = &self.bearer_fp {
            object.insert("bearer_fp".into(), Value::String(bearer_fp.clone()));
        }
    }
}

/// Why the gate refused a request. Logged, never sent to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthFailure {
    MissingBearer,
    /// The Bearer is not the slot's current Live ticket (stale, rotated away, or wrong).
    TicketMismatch,
    /// The slot's ticket was revoked and not replaced.
    TicketRevoked,
    SlotUnknown,
    StoreUnavailable,
}

impl AuthFailure {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::MissingBearer => "missing_bearer",
            Self::TicketMismatch => "ticket_mismatch",
            Self::TicketRevoked => "ticket_revoked",
            Self::SlotUnknown => "slot_unknown",
            Self::StoreUnavailable => "store_unavailable",
        }
    }
}

/// The failure and, when the slot has a Live ticket, its fingerprint: next to
/// the request's `bearer_fp` it shows whether the client holds the current one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuthDiagnosis {
    pub(super) reason: AuthFailure,
    pub(super) live_fp: Option<String>,
}

impl AuthDiagnosis {
    fn new(reason: AuthFailure) -> Self {
        Self { reason, live_fp: None }
    }

    pub(super) fn add_to(&self, object: &mut Map<String, Value>) {
        object.insert("auth_reason".into(), Value::String(self.reason.as_str().into()));
        if let Some(live_fp) = &self.live_fp {
            object.insert("live_fp".into(), Value::String(live_fp.clone()));
        }
    }
}

/// Explain a refused request from what the gate saw. Pure: the caller reads
/// the ledger.
pub(super) fn diagnose_bearer(
    slot: Result<Slot, OAuthError>,
    presented: Option<&str>,
    record: Result<Option<LedgerRecord>, OAuthError>,
) -> AuthDiagnosis {
    if slot.is_err() {
        return AuthDiagnosis::new(AuthFailure::SlotUnknown);
    }
    if presented.is_none() {
        return AuthDiagnosis::new(AuthFailure::MissingBearer);
    }
    match record {
        Err(_) => AuthDiagnosis::new(AuthFailure::StoreUnavailable),
        Ok(None) => AuthDiagnosis::new(AuthFailure::TicketMismatch),
        Ok(Some(record)) if record.state == TicketState::Revoked => {
            AuthDiagnosis::new(AuthFailure::TicketRevoked)
        }
        Ok(Some(record)) => AuthDiagnosis {
            reason: AuthFailure::TicketMismatch,
            live_fp: Some(bearer_fingerprint(record.handle.as_str())),
        },
    }
}

/// [`diagnose_bearer`] for a request that failed `verify_registered_slot`.
pub(super) fn diagnose_slot_bearer(scene_slot: &str, headers: &HeaderMap) -> AuthDiagnosis {
    let slot = Slot::parse(scene_slot);
    let record = match slot {
        Ok(slot) => ledger_record(slot),
        Err(_) => Ok(None),
    };
    diagnose_bearer(slot, bearer_secret(headers), record)
}

#[cfg(test)]
#[path = "../../unit-tests/mcp_host/caller_tests.rs"]
mod tests;
