//! Host-produced MCP OAuth consent HTML. No HTTP bind.

const PAGE: &str = include_str!("oauth_consent.html");
const SHEET: &str = include_str!("oauth_consent.css");
const SCRIPT: &str = include_str!("oauth_consent.js");

/// Consent page for the default client name.
pub fn oauth_consent_html() -> String {
    oauth_consent_html_for("Codex")
}

/// Consent page naming `client`. The name is untrusted, so it is escaped.
pub fn oauth_consent_html_for(client: &str) -> String {
    PAGE.replace("{{client}}", &escape_html(client))
        .replace("<style id=\"sheet\"></style>", &format!("<style>{SHEET}</style>"))
        .replace(
            "<script id=\"behavior\"></script>",
            &format!("<script>{SCRIPT}</script>"),
        )
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
#[path = "../unit-tests/services/oauth_consent.rs"]
mod tests;
