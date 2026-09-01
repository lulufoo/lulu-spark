use std::fs;

use chrono::Utc;

use super::store::agent_log_dir;

/// Redact secrets before writing agent error logs.
pub fn redact_secrets(message: &str) -> String {
    let mut out = message.to_string();
    // Authorization: Bearer …
    if let Some(idx) = out.to_ascii_lowercase().find("authorization:") {
        let rest = &out[idx..];
        let end = rest.find(['\n', '\r']).unwrap_or(rest.len());
        let replacement = "Authorization: [REDACTED]";
        out.replace_range(idx..idx + end, replacement);
    }
    // Bearer tokens elsewhere
    while let Some(idx) = out.to_ascii_lowercase().find("bearer ") {
        let start = idx;
        let rest = &out[start + "bearer ".len()..];
        let end_rel = rest
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
            .unwrap_or(rest.len());
        let end = start + "bearer ".len() + end_rel;
        out.replace_range(start..end, "Bearer [REDACTED]");
    }
    // api_key=… / "api_key":"…"
    let patterns = ["api_key=", "\"api_key\":\""];
    for pat in patterns {
        let lower = out.to_ascii_lowercase();
        if let Some(idx) = lower.find(pat) {
            let value_start = idx + pat.len();
            let rest = &out[value_start..];
            let end_rel = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
                .unwrap_or(rest.len());
            out.replace_range(value_start..value_start + end_rel, "[REDACTED]");
        }
    }
    out
}

pub fn log_agent_error(message: &str) -> Result<(), String> {
    let dir = agent_log_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("agent-error.log");
    let line = format!(
        "{} {}\n",
        Utc::now().to_rfc3339(),
        redact_secrets(message)
    );
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}
