//! L4 authorization-code grant + PKCE. No HTTP bind. Not the MCP Host.

mod audit;
mod client;
mod pkce;
mod secret;

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::services::mcp_oauth::{self, Slot};

use client::{build_client, display_name, push_bounded, FALLBACK_NAME};
pub use client::{valid_redirect_uri, RegisteredClient};
use pkce::{is_verifier, verify_pkce};
pub(super) use pkce::s256;
use secret::new_code;

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
    invalid_redirect_uri,
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
    clients: Mutex<Vec<RegisteredClient>>,
}

impl Auth {
    pub fn new() -> Self {
        Self {
            codes: Mutex::new(HashMap::new()),
            clients: Mutex::new(Vec::new()),
        }
    }

    /// RFC 7591 registration for a public client. Returns the new `client_id`.
    pub fn register_client(
        &self,
        client_name: &str,
        redirect_uris: &[String],
    ) -> Result<RegisteredClient, AuthError> {
        let client = build_client(client_name, redirect_uris)?;
        let mut clients = self.clients.lock().map_err(|_| AuthError::rejected)?;
        push_bounded(&mut clients, client.clone());
        Ok(client)
    }

    /// Checks an authorize request and returns the name to show on the consent page.
    ///
    /// A registered client must use one of its registered redirect URIs, exactly.
    /// An unknown `client_id` (registry is memory only, so a restart forgets it)
    /// is accepted only when the redirect URI passes the same shape rule as registration.
    pub fn authorize_client(
        &self,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<String, AuthError> {
        if client_id.is_empty() || !valid_redirect_uri(redirect_uri) {
            let reason = if client_id.is_empty() {
                "client_id_missing"
            } else {
                "redirect_uri_invalid"
            };
            audit::authorize_rejected(client_id, redirect_uri, reason);
            return Err(AuthError::invalid_request);
        }
        let clients = self.clients.lock().map_err(|_| AuthError::rejected)?;
        match clients.iter().find(|c| c.client_id == client_id) {
            Some(client) if client.redirect_uris.iter().any(|u| u == redirect_uri) => {
                Ok(display_name(&client.client_name))
            }
            Some(_) => {
                audit::authorize_rejected(client_id, redirect_uri, "redirect_uri_not_registered");
                Err(AuthError::invalid_request)
            }
            None => {
                audit::unknown_client(client_id, redirect_uri);
                Ok(FALLBACK_NAME.to_string())
            }
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
        let client_id = request.client_id.clone();
        match self.exchange(request) {
            Ok((grant, slot)) => {
                audit::token_issued(&client_id, slot);
                Ok(grant)
            }
            Err((error, reason)) => {
                audit::token_rejected(&client_id, reason);
                Err(error)
            }
        }
    }

    /// The check chain. Each refusal carries the reason that goes to the log only.
    fn exchange(
        &self,
        request: TokenRequest,
    ) -> Result<(TokenGrant, Slot), (AuthError, &'static str)> {
        if request.grant_type != "authorization_code" {
            return Err((AuthError::unsupported_grant, "unsupported_grant_type"));
        }
        if request.code.is_empty() || request.code_verifier.is_empty() {
            return Err((AuthError::invalid_request, "code_or_verifier_missing"));
        }
        if !is_verifier(&request.code_verifier) {
            return Err((AuthError::invalid_request, "verifier_shape_invalid"));
        }
        let row = self
            .codes
            .lock()
            .map_err(|_| (AuthError::rejected, "code_store_unavailable"))?
            .remove(&request.code)
            .ok_or((AuthError::invalid_grant, "code_unknown_or_used"))?;
        if Instant::now() >= row.expires_at {
            return Err((AuthError::invalid_grant, "code_expired"));
        }
        if row.client_id != request.client_id {
            return Err((AuthError::invalid_grant, "client_id_mismatch"));
        }
        if row.redirect_uri != request.redirect_uri {
            return Err((AuthError::invalid_grant, "redirect_uri_mismatch"));
        }
        if !verify_pkce(row.method, &row.challenge, &request.code_verifier) {
            return Err((AuthError::invalid_grant, "pkce_mismatch"));
        }
        let handle = mcp_oauth::issue_for_slot(row.slot)
            .map_err(|_| (AuthError::rejected, "ticket_issue_failed"))?;
        let grant = TokenGrant {
            access_token: handle.as_str().to_string(),
            token_type: "Bearer",
        };
        Ok((grant, row.slot))
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

#[cfg(test)]
#[path = "../../unit-tests/services/auth.rs"]
mod tests;
