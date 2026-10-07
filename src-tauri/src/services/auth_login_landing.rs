//! Host-produced auth-login landing HTML. No HTTP bind.

use std::path::PathBuf;

const FALLBACK_HTML: &str = include_str!("auth_login_landing.html");

fn landing_html_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/auth_login_landing.html")
}

pub fn auth_login_landing_html() -> String {
    std::fs::read_to_string(landing_html_path()).unwrap_or_else(|_| FALLBACK_HTML.to_string())
}

#[cfg(test)]
#[path = "../unit-tests/services/auth_login_landing.rs"]
mod tests;
