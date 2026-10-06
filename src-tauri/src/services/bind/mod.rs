//! In-process bind ceremony: one ephemeral X25519 session, Ed25519 signed payload.

use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::Value;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

#[cfg(test)]
use crate::config::vault;
use crate::services::mcp_oauth::issue_for_device;

const BIND_TTL_SECS: u64 = 180;
const SEAL_INFO: &[u8] = b"lulu-spark-bind-v1";
pub const BIND_MOBILE_BUSINESS_ID: &str = "Bind_Mobile";

pub fn log_bind_event(event: &'static str, outcome: &'static str) {
    eprintln!(
        "[bind] business_id={} event={} outcome={}",
        BIND_MOBILE_BUSINESS_ID, event, outcome
    );
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindError {
    session_unavailable,
    expired,
    consumed,
    decrypt_failed,
    invalid_request,
    keychain_unavailable,
    rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindPayload {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub temp_pub: String,
    pub tls_fingerprint: String,
    pub exp: u64,
    pub sign_pub: String,
    pub sig: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindResult {
    pub device_mcp_token: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindSessionState
{
    idle,
    live,
    consumed,
    expired,
}

enum Session {
    Live { secret: [u8; 32], exp: u64 },
    Consumed,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(text: &str) -> Result<Vec<u8>, BindError> {
    if text.len() % 2 != 0 {
        return Err(BindError::invalid_request);
    }
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|_| BindError::invalid_request))
        .collect()
}

fn hex_decode_32(text: &str) -> Result<[u8; 32], BindError> {
    let bytes = hex_decode(text)?;
    bytes.try_into().map_err(|_| BindError::invalid_request)
}

fn session_lock() -> std::sync::MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn canonical_bind_string(payload: &BindPayload) -> String {
    format!(
        "v1|{}|{}|{}|{}|{}",
        payload.ip, payload.port, payload.temp_pub, payload.tls_fingerprint, payload.exp
    )
}

pub fn verify_bind_signature(payload: &BindPayload) -> Result<(), BindError> {
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&hex_decode_32(&payload.sign_pub)?)
        .map_err(|_| BindError::rejected)?;
    let sig_bytes: [u8; 64] = hex_decode(&payload.sig)?
        .try_into()
        .map_err(|_| BindError::rejected)?;
    let sig = Signature::from_bytes(&sig_bytes);
    verifying
        .verify(canonical_bind_string(payload).as_bytes(), &sig)
        .map_err(|_| BindError::rejected)
}

fn derive_seal_key(shared: &[u8; 32]) -> Result<[u8; 32], BindError> {
    let hk = Hkdf::<Sha256>::new(None, shared);
    let mut okm = [0u8; 32];
    hk.expand(SEAL_INFO, &mut okm)
        .map_err(|_| BindError::decrypt_failed)?;
    Ok(okm)
}

pub fn seal_bind_request(
    temp_pub_hex: &str,
    device_id: &str,
    device_label: Option<&str>,
) -> Result<Vec<u8>, BindError> {
    let host_pub = PublicKey::from(hex_decode_32(temp_pub_hex)?);
    let eph = StaticSecret::random_from_rng(OsRng);
    let eph_pub = PublicKey::from(&eph);
    let shared = eph.diffie_hellman(&host_pub);
    let key = derive_seal_key(shared.as_bytes())?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| BindError::decrypt_failed)?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let mut body = serde_json::json!({ "v": "1", "device_id": device_id });
    if let Some(label) = device_label {
        body["device_label"] = Value::String(label.to_string());
    }
    let plaintext = serde_json::to_vec(&body).map_err(|_| BindError::invalid_request)?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_ref())
        .map_err(|_| BindError::decrypt_failed)?;
    let mut out = Vec::with_capacity(32 + 12 + ct.len());
    out.extend_from_slice(eph_pub.as_bytes());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn open_bind_request(secret: &[u8; 32], encrypted: &[u8]) -> Result<Value, BindError> {
    if encrypted.len() < 32 + 12 + 16 {
        return Err(BindError::session_unavailable);
    }
    let eph_pub = PublicKey::from(
        <[u8; 32]>::try_from(&encrypted[..32]).map_err(|_| BindError::session_unavailable)?,
    );
    let nonce = &encrypted[32..44];
    let ct = &encrypted[44..];
    let host = StaticSecret::from(*secret);
    let shared = host.diffie_hellman(&eph_pub);
    let key = derive_seal_key(shared.as_bytes())?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| BindError::session_unavailable)?;
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| BindError::session_unavailable)?;
    serde_json::from_slice(&plain).map_err(|_| BindError::invalid_request)
}

pub fn create_bind_payload(
    ip: Ipv4Addr,
    port: u16,
    tls_fingerprint: &str,
) -> Result<BindPayload, BindError> {
    log_bind_event("payload.create", "started");
    let secret = StaticSecret::random_from_rng(OsRng);
    let temp_pub = hex_encode(PublicKey::from(&secret).as_bytes());
    let exp = now_secs() + BIND_TTL_SECS;
    let ephemeral = SigningKey::generate(&mut OsRng);
    let payload = BindPayload {
        ip,
        port,
        temp_pub,
        tls_fingerprint: tls_fingerprint.to_string(),
        exp,
        sign_pub: hex_encode(ephemeral.verifying_key().as_bytes()),
        sig: String::new(),
    };
    let sig = ephemeral.sign(canonical_bind_string(&payload).as_bytes());
    let payload = BindPayload {
        sig: hex_encode(&sig.to_bytes()),
        ..payload
    };
    *session_lock() = Some(Session::Live {
        secret: secret.to_bytes(),
        exp,
    });
    log_bind_event("payload.create", "succeeded");
    Ok(payload)
}

pub fn complete_bind(encrypted_request: &[u8]) -> Result<BindResult, BindError> {
    let mut guard = session_lock();
    let secret = match guard.as_ref() {
        None => return Err(BindError::session_unavailable),
        Some(Session::Consumed) => return Err(BindError::consumed),
        Some(Session::Live { exp, .. }) if now_secs() > *exp => {
            *guard = None;
            return Err(BindError::expired);
        }
        Some(Session::Live { secret, .. }) => *secret,
    };
    let body = open_bind_request(&secret, encrypted_request)?;
    let v = body.get("v").and_then(Value::as_str).unwrap_or("");
    let device_id = body
        .get("device_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if v != "1" || device_id.is_empty() {
        return Err(BindError::invalid_request);
    }
    let device_label = body.get("device_label").and_then(Value::as_str);
    let token = issue_for_device(device_id, device_label).map_err(|_| BindError::rejected)?;
    *guard = Some(Session::Consumed);
    Ok(BindResult {
        device_mcp_token: token.as_str().to_string(),
    })
}

pub fn bind_session_state() -> BindSessionState
{
    match session_lock().as_ref() {
        None => BindSessionState::idle,
        Some(Session::Consumed) => BindSessionState::consumed,
        Some(Session::Live { exp, .. }) if now_secs() <= *exp => BindSessionState::live,
        Some(Session::Live { .. }) => BindSessionState::expired,
    }
}

#[cfg(test)]
pub fn test_clear_session() {
    *session_lock() = None;
}

#[cfg(test)]
pub fn test_expire_current_session() {
    let mut guard = session_lock();
    if let Some(Session::Live { secret, .. }) = guard.as_ref() {
        let secret = *secret;
        *guard = Some(Session::Live { secret, exp: 0 });
    }
}

#[cfg(test)]
pub fn test_set_current_session_exp(exp: u64) {
    let mut guard = session_lock();
    if let Some(Session::Live { secret, .. }) = guard.as_ref() {
        let secret = *secret;
        *guard = Some(Session::Live { secret, exp });
    }
}

#[cfg(test)]
pub fn test_session_bytes() -> Vec<u8> {
    match session_lock().as_ref() {
        None => Vec::new(),
        Some(Session::Consumed) => vec![1],
        Some(Session::Live { secret, exp }) => {
            let mut out = Vec::with_capacity(41);
            out.push(2);
            out.extend_from_slice(secret);
            out.extend_from_slice(&exp.to_le_bytes());
            out
        }
    }
}

#[cfg(test)]
pub fn test_reset_bind_keychain() {
    vault::test_clear_legacy_bind_accounts();
}

#[cfg(test)]
pub fn test_put_bind_account(account: &str, value: &str) {
    vault::test_seed_legacy_bind_account(account, value);
}

#[cfg(test)]
pub fn test_bind_account(account: &str) -> Option<String> {
    vault::test_legacy_bind_account(account)
}

#[cfg(test)]
#[path = "../../unit-tests/services/bind.rs"]
mod tests;
