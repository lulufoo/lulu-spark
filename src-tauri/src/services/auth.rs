//! L4 authorization-code grant + PKCE. No HTTP bind. Not the MCP Host.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::services::mcp_oauth::{self, Slot};

const CODE_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeMethod {
    S256,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    invalid_request,
    invalid_grant,
    unsupported_grant,
    rejected,
}

struct IssuedCode {
    challenge: String,
    method: ChallengeMethod,
    redirect_uri: String,
    client_id: String,
    slot: Slot,
    expires_at: Instant,
}

pub struct CodeIssue {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: ChallengeMethod,
    pub slot: Slot,
    pub ttl: Duration,
}

impl CodeIssue {
    pub fn s256(
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
        code_challenge: impl Into<String>,
        slot: Slot,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            redirect_uri: redirect_uri.into(),
            code_challenge: code_challenge.into(),
            code_challenge_method: ChallengeMethod::S256,
            slot,
            ttl: CODE_TTL,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
}

pub struct TokenRequest {
    pub grant_type: String,
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    pub client_id: String,
}

pub struct TokenGrant {
    pub access_token: String,
    pub token_type: &'static str,
}

impl fmt::Debug for TokenGrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenGrant")
            .field("access_token", &"[REDACTED]")
            .field("token_type", &self.token_type)
            .finish()
    }
}

pub struct Auth {
    codes: Mutex<HashMap<String, IssuedCode>>,
}

impl Auth {
    pub fn new() -> Self {
        Self {
            codes: Mutex::new(HashMap::new()),
        }
    }

    pub fn issue_code(&self, issue: CodeIssue) -> Result<String, AuthError> {
        if issue.client_id.is_empty()
            || issue.redirect_uri.is_empty()
            || issue.code_challenge.is_empty()
        {
            return Err(AuthError::invalid_request);
        }
        if !matches!(issue.code_challenge_method, ChallengeMethod::S256) {
            return Err(AuthError::invalid_request);
        }
        let code = new_code();
        let row = IssuedCode {
            challenge: issue.code_challenge,
            method: issue.code_challenge_method,
            redirect_uri: issue.redirect_uri,
            client_id: issue.client_id,
            slot: issue.slot,
            expires_at: Instant::now() + issue.ttl,
        };
        self.codes
            .lock()
            .map_err(|_| AuthError::rejected)?
            .insert(code.clone(), row);
        Ok(code)
    }

    pub fn exchange_token(&self, request: TokenRequest) -> Result<TokenGrant, AuthError> {
        if request.grant_type != "authorization_code" {
            return Err(AuthError::unsupported_grant);
        }
        if request.code.is_empty() || request.code_verifier.is_empty() {
            return Err(AuthError::invalid_request);
        }
        if !is_verifier(&request.code_verifier) {
            return Err(AuthError::invalid_request);
        }
        let row = self
            .codes
            .lock()
            .map_err(|_| AuthError::rejected)?
            .remove(&request.code)
            .ok_or(AuthError::invalid_grant)?;
        if Instant::now() >= row.expires_at {
            return Err(AuthError::invalid_grant);
        }
        if row.client_id != request.client_id || row.redirect_uri != request.redirect_uri {
            return Err(AuthError::invalid_grant);
        }
        if !verify_pkce(row.method, &row.challenge, &request.code_verifier) {
            return Err(AuthError::invalid_grant);
        }
        let handle = mcp_oauth::issue_for_slot(row.slot).map_err(|_| AuthError::rejected)?;
        Ok(TokenGrant {
            access_token: handle.as_str().to_string(),
            token_type: "Bearer",
        })
    }
}

pub fn shared() -> &'static Auth {
    static AUTH: OnceLock<Auth> = OnceLock::new();
    AUTH.get_or_init(Auth::new)
}

pub fn token_body(grant: &TokenGrant) -> Value {
    json!({
        "access_token": grant.access_token,
        "token_type": grant.token_type,
    })
}

fn verify_pkce(method: ChallengeMethod, challenge: &str, verifier: &str) -> bool {
    match method {
        ChallengeMethod::S256 => s256(verifier) == challenge,
    }
}

fn s256(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn is_verifier(value: &str) -> bool {
    let len = value.len();
    (43..=128).contains(&len)
        && value
            .bytes()
            .all(|b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'))
}

fn new_code() -> String {
    let mut bytes = [0u8; 32];
    if !read_random_bytes(&mut bytes) {
        fill_weak_random(&mut bytes);
    }
    URL_SAFE_NO_PAD.encode(bytes)
}

fn read_random_bytes(buf: &mut [u8]) -> bool {
    use std::io::Read;
    #[cfg(unix)]
    {
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            return file.read_exact(buf).is_ok();
        }
    }
    let _ = buf;
    false
}

fn fill_weak_random(buf: &mut [u8]) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    for (index, slot) in buf.iter_mut().enumerate() {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        index.hash(&mut hasher);
        *slot = (hasher.finish() & 0xFF) as u8;
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/auth.rs"]
mod tests;
