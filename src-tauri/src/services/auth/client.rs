//! RFC 7591 client registry for public MCP clients. In memory, bounded.

use super::secret::new_code;
use super::AuthError;

const MAX_CLIENTS: usize = 256;
const MAX_REDIRECT_URIS: usize = 8;
const MAX_REDIRECT_CHARS: usize = 2048;
const MAX_NAME_CHARS: usize = 80;
pub(super) const FALLBACK_NAME: &str = "MCP client";

/// Schemes that must never receive an authorization code.
/// `spark` is this app's own deep link, so a code sent there would loop back into Spark.
const DENIED_SCHEMES: &[&str] = &[
    "spark", "javascript", "data", "file", "vbscript", "about", "blob", "ftp", "ws", "wss",
    "chrome", "view-source",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredClient {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

pub(super) fn build_client(
    name: &str,
    redirect_uris: &[String],
) -> Result<RegisteredClient, AuthError> {
    if redirect_uris.is_empty() || redirect_uris.len() > MAX_REDIRECT_URIS {
        return Err(AuthError::invalid_redirect_uri);
    }
    // RFC 7591 §3.2.1 lets the server replace requested values. A client may list URIs this
    // server will never send a code to (Cursor adds its https cloud callback next to the
    // loopback one), so keep the usable ones and refuse only when none is left.
    let mut unique: Vec<String> = Vec::new();
    for uri in redirect_uris {
        if valid_redirect_uri(uri) && !unique.contains(uri) {
            unique.push(uri.clone());
        }
    }
    if unique.is_empty() {
        return Err(AuthError::invalid_redirect_uri);
    }
    Ok(RegisteredClient {
        client_id: format!("lulu-{}", &new_code()[..22]),
        client_name: display_name(name),
        redirect_uris: unique,
    })
}

/// Oldest rows leave first, so a client that re-registers on each connect never fills it.
pub(super) fn push_bounded(clients: &mut Vec<RegisteredClient>, client: RegisteredClient) {
    if clients.len() >= MAX_CLIENTS {
        clients.remove(0);
    }
    clients.push(client);
}

pub(super) fn display_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        FALLBACK_NAME.to_string()
    } else {
        clean.to_string()
    }
}

/// Loopback http(s), or a private-use app scheme (RFC 8252). No fragment.
pub fn valid_redirect_uri(uri: &str) -> bool {
    if uri.is_empty() || uri.chars().count() > MAX_REDIRECT_CHARS {
        return false;
    }
    if uri.chars().any(|c| c.is_control() || c.is_whitespace()) || uri.contains('#') {
        return false;
    }
    let Some((scheme, rest)) = uri.split_once(':') else {
        return false;
    };
    let scheme = scheme.to_ascii_lowercase();
    if !is_scheme(&scheme) || rest.is_empty() {
        return false;
    }
    if scheme == "http" || scheme == "https" {
        return is_loopback_authority(rest);
    }
    !DENIED_SCHEMES.contains(&scheme.as_str())
}

fn is_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "+.-".contains(c))
}

fn is_loopback_authority(rest: &str) -> bool {
    let Some(rest) = rest.strip_prefix("//") else {
        return false;
    };
    let authority = rest.split(['/', '?']).next().unwrap_or("");
    if authority.contains('@') {
        return false;
    }
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if port.is_some_and(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }
    host == "127.0.0.1" || host == "localhost"
}

#[cfg(test)]
#[path = "../../unit-tests/services/auth_client.rs"]
mod tests;
