use std::path::Path;

use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};

use super::document::create_note_from_markdown;

/// Options for Host-side jot shell synthesis.
/// Topic is always `inbox`; `source_type` is always `jot` — callers cannot override either.
#[derive(Debug, Clone, Default)]
pub struct JotCreateOpts {
    /// Fixed `YYYYMMDDHHMM` (UTC+8) for tests; `None` uses current time.
    pub ts: Option<String>,
}

fn now_ts_utc8() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    Utc::now()
        .with_timezone(&tz)
        .format("%Y%m%d%H%M")
        .to_string()
}

fn format_created_at_zh(ts: &str) -> Result<String, String> {
    if ts.len() != 12 || !ts.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid ts: {ts}"));
    }
    let year: u32 = ts[0..4]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let month: u32 = ts[4..6]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let day: u32 = ts[6..8]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let hour = &ts[8..10];
    let minute = &ts[10..12];
    Ok(format!("{year}年{month}月{day}日 {hour}:{minute}"))
}

fn slugify_title(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = true;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "jot".to_string()
    } else {
        out
    }
}

/// Build a minimal legal archive MD shell for a jot (H1 / 创建时间 / digest nav).
/// No persistence. Topic is fixed to `inbox`.
pub fn synthesize_jot_document(body: &str, opts: &JotCreateOpts) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    let ts = opts.ts.clone().unwrap_or_else(now_ts_utc8);
    let first = body.lines().next().unwrap_or("").trim();
    let slug = if first.is_empty() {
        "jot".to_string()
    } else {
        slugify_title(first)
    };
    let title = if first.is_empty() {
        format!("{ts}-{slug}")
    } else {
        first.to_string()
    };
    let created_at_zh = format_created_at_zh(&ts)?;
    let common_path = format!("inbox/notes/{ts}-{slug}.md");
    Ok(format!(
        "# {title}\n\n\
         > 创建时间：{created_at_zh}\n\
         > 来源：jot\n\
         > 导航：[digest](../../../digest/{common_path})\n\n\
         ---\n\n\
         {body}"
    ))
}

/// Synthesize jot shell then append-only write via private markdown API.
pub fn create_jot(
    repo_root: &Path,
    body: &str,
    opts: &JotCreateOpts,
) -> Result<Value, String> {
    let document = synthesize_jot_document(body, opts)?;
    let payload = json!({ "source_type": "jot" });
    let result = create_note_from_markdown(repo_root, &document, &payload);
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
