//! Host-produced MCP OAuth consent HTML. No HTTP bind.

const PAGE: &str = include_str!("oauth_consent.html");
const SHEET: &str = include_str!("oauth_consent.css");
const SCRIPT: &str = include_str!("oauth_consent.js");

pub fn oauth_consent_html() -> String {
    PAGE.replace("<style id=\"sheet\"></style>", &format!("<style>{SHEET}</style>"))
        .replace(
            "<script id=\"behavior\"></script>",
            &format!("<script>{SCRIPT}</script>"),
        )
}

#[cfg(test)]
#[path = "../unit-tests/services/oauth_consent.rs"]
mod tests;
