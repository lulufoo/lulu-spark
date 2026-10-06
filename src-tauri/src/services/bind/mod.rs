//! In-process bind ceremony: one ephemeral RSA-2048 session.
//! PSS-SHA256 signs the QR; OAEP-SHA256 wraps a ChaCha20-Poly1305 key.

use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::rngs::OsRng;
use rand::RngCore;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use rsa::pss::{BlindedSigningKey, Signature as PssSignature, VerifyingKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding, Verifier};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde_json::Value;
use sha2::Sha256;

use crate::services::mcp_oauth::issue_for_device;

const BIND_TTL_SECS: u64 = 180;
const RSA_BITS: usize = 2048;
const OAEP_CT_LEN: usize = 256;
const NONCE_LEN: usize = 12;
const SYM_KEY_LEN: usize = 32;
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
    rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindPayload {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub temp_pub: String,
    pub tls_fingerprint: String,
    pub exp: u64,
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
    Live { secret: Vec<u8>, exp: u64 },
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

fn session_lock() -> std::sync::MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn canonical_bind_string(payload: &BindPayload) -> String {
    format!(
        "v1|{}|{}|{}|{}|{}",
        payload.ip, payload.port, payload.temp_pub, payload.tls_fingerprint, payload.exp
    )
}

fn rsa_public(temp_pub_hex: &str) -> Result<RsaPublicKey, BindError> {
    RsaPublicKey::from_public_key_der(&hex_decode(temp_pub_hex)?).map_err(|_| BindError::rejected)
}

pub fn verify_bind_signature(payload: &BindPayload) -> Result<(), BindError> {
    let verifying = VerifyingKey::<Sha256>::new(rsa_public(&payload.temp_pub)?);
    let sig = PssSignature::try_from(hex_decode(&payload.sig)?.as_slice()).map_err(|_| BindError::rejected)?;
    verifying
        .verify(canonical_bind_string(payload).as_bytes(), &sig)
        .map_err(|_| BindError::rejected)
}

fn seal_plain(device_id: &str, device_label: Option<&str>) -> Result<Vec<u8>, BindError> {
    let mut body = serde_json::json!({ "v": "1", "device_id": device_id });
    if let Some(label) = device_label {
        body["device_label"] = Value::String(label.to_string());
    }
    serde_json::to_vec(&body).map_err(|_| BindError::invalid_request)
}

fn aead_crypt(encrypt: bool, key: &[u8], nonce: &[u8], input: &[u8]) -> Result<Vec<u8>, BindError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| BindError::decrypt_failed)?;
    let n = Nonce::from_slice(nonce);
    if encrypt {
        cipher.encrypt(n, input).map_err(|_| BindError::decrypt_failed)
    } else {
        cipher.decrypt(n, input).map_err(|_| BindError::session_unavailable)
    }
}

pub fn seal_bind_request(
    temp_pub_hex: &str,
    device_id: &str,
    device_label: Option<&str>,
) -> Result<Vec<u8>, BindError> {
    let public_key = rsa_public(temp_pub_hex).map_err(|_| BindError::invalid_request)?;
    let mut key = [0u8; SYM_KEY_LEN];
    OsRng.fill_bytes(&mut key);
    let wrapped = public_key
        .encrypt(&mut OsRng, Oaep::new::<Sha256>(), &key)
        .map_err(|_| BindError::decrypt_failed)?;
    if wrapped.len() != OAEP_CT_LEN {
        return Err(BindError::decrypt_failed);
    }
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ct = aead_crypt(true, &key, &nonce, &seal_plain(device_id, device_label)?)?;
    let mut out = Vec::with_capacity(OAEP_CT_LEN + NONCE_LEN + ct.len());
    out.extend_from_slice(&wrapped);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn open_bind_request(secret: &[u8], encrypted: &[u8]) -> Result<Value, BindError> {
    if encrypted.len() < OAEP_CT_LEN + NONCE_LEN + 16 {
        return Err(BindError::session_unavailable);
    }
    let private_key =
        RsaPrivateKey::from_pkcs8_der(secret).map_err(|_| BindError::session_unavailable)?;
    let key = private_key
        .decrypt(Oaep::new::<Sha256>(), &encrypted[..OAEP_CT_LEN])
        .map_err(|_| BindError::session_unavailable)?;
    let nonce = &encrypted[OAEP_CT_LEN..OAEP_CT_LEN + NONCE_LEN];
    let plain = aead_crypt(false, &key, nonce, &encrypted[OAEP_CT_LEN + NONCE_LEN..])?;
    serde_json::from_slice(&plain).map_err(|_| BindError::invalid_request)
}

pub fn create_bind_payload(
    ip: Ipv4Addr,
    port: u16,
    tls_fingerprint: &str,
) -> Result<BindPayload, BindError> {
    log_bind_event("payload.create", "started");
    let private_key = RsaPrivateKey::new(&mut OsRng, RSA_BITS).map_err(|_| BindError::rejected)?;
    let temp_pub = hex_encode(
        RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .map_err(|_| BindError::rejected)?
            .as_bytes(),
    );
    let exp = now_secs() + BIND_TTL_SECS;
    let mut payload = BindPayload {
        ip,
        port,
        temp_pub,
        tls_fingerprint: tls_fingerprint.to_string(),
        exp,
        sig: String::new(),
    };
    let signing = BlindedSigningKey::<Sha256>::new(private_key.clone());
    payload.sig = hex_encode(
        &signing
            .sign_with_rng(&mut OsRng, canonical_bind_string(&payload).as_bytes())
            .to_bytes(),
    );
    *session_lock() = Some(Session::Live {
        secret: private_key
            .to_pkcs8_der()
            .map_err(|_| BindError::rejected)?
            .as_bytes()
            .to_vec(),
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
        Some(Session::Live { secret, .. }) => secret.clone(),
    };
    let body = open_bind_request(&secret, encrypted_request)?;
    let v = body.get("v").and_then(Value::as_str).unwrap_or("");
    let device_id = body.get("device_id").and_then(Value::as_str).unwrap_or("");
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
fn replace_live_exp(exp: u64) {
    let mut guard = session_lock();
    if let Some(Session::Live { secret, .. }) = guard.as_ref() {
        let secret = secret.clone();
        *guard = Some(Session::Live { secret, exp });
    }
}

#[cfg(test)]
pub fn test_expire_current_session() {
    replace_live_exp(0);
}

#[cfg(test)]
pub fn test_set_current_session_exp(exp: u64) {
    replace_live_exp(exp);
}

#[cfg(test)]
pub fn test_session_bytes() -> Vec<u8> {
    match session_lock().as_ref() {
        None => Vec::new(),
        Some(Session::Consumed) => vec![1],
        Some(Session::Live { secret, exp }) => {
            let mut out = Vec::with_capacity(1 + secret.len() + 8);
            out.push(2);
            out.extend_from_slice(secret);
            out.extend_from_slice(&exp.to_le_bytes());
            out
        }
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/bind.rs"]
mod tests;
