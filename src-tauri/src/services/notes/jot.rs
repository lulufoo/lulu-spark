use std::path::Path;

use serde_json::{json, Value};

use super::create_meta::format_created_at_zh;
use super::document::create_note_from_markdown;

/// Options for Host-side jot shell synthesis.
/// Topic is always `inbox`; `source_type` is always `jot` — callers cannot override either.
#[derive(Debug, Clone, Default)]
pub struct JotCreateOpts {
    /// Fixed `YYYYMMDDHHMM` (UTC+8) for tests; `None` uses current time.
    pub ts: Option<String>,
}

/// Build a minimal archive MD shell for a jot (H1 / 创建时间). No navigation.
/// No persistence. Topic is fixed to `inbox`.
pub fn synthesize_jot_document(body: &str, opts: &JotCreateOpts) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    let ts = opts.ts.clone().unwrap_or_else(|| {
        chrono::Utc::now()
            .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).expect("UTC+8"))
            .format("%Y%m%d%H%M")
            .to_string()
    });
    let first = body.lines().next().unwrap_or("").trim();
    let title = if first.is_empty() {
        ts.clone()
    } else {
        first.to_string()
    };
    let created_at_zh = format_created_at_zh(&ts)?;
    Ok(format!(
        "# {title}\n\n> 创建时间：{created_at_zh}\n\n---\n\n{body}"
    ))
}

/// Write a jot via create-note-content rules (`inbox/notes/{ts}-{rand}.md`).
pub fn create_jot(
    repo_root: &Path,
    body: &str,
    opts: &JotCreateOpts,
) -> Result<Value, String> {
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    let first = body.lines().next().unwrap_or("").trim();
    let ts = opts.ts.clone().unwrap_or_default();
    let title = if first.is_empty() {
        if ts.is_empty() {
            "jot".to_string()
        } else {
            ts.clone()
        }
    } else {
        first.to_string()
    };
    let mut payload = json!({
        "title": title,
        "source_type": "jot",
        "project": "inbox",
        "theme": "notes",
    });
    if !ts.is_empty() {
        payload["created_at"] = json!(ts);
    }
    let result = create_note_from_markdown(repo_root, body, &payload);
    if result.get("ok") == Some(&json!(true)) {
        Ok(result)
    } else {
        let status = result
            .get("_status")
            .and_then(|v| v.as_u64())
            .unwrap_or(500);
        let error = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("archive failed");
        Err(format!("{status}: {error}"))
    }
}
