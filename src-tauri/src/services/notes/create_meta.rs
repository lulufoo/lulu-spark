//! Shared create-note metadata: title, path segments, source_type, raw shell.

use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};

use crate::services::archive_parse::is_valid_common_path;
use crate::services::id::random_alnum6;

const SOURCE_TYPES: &[&str] = &[
    "summary",
    "article",
    "theme-line",
    "dialogue",
    "transcript",
    "jot",
];

pub(super) struct CreateMeta {
    pub title: String,
    pub created_at: String,
    pub source_type: String,
    pub common_path: String,
}

pub(super) fn parse_create_meta(
    payload: &Value,
    source_basename: Option<&str>,
) -> Result<CreateMeta, Value> {
    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| json!({ "error": "Missing title", "_status": 400 }))?
        .to_string();

    let project = path_segment(payload, "project", "inbox")?;
    let theme = path_segment(payload, "theme", "notes")?;
    let created_at = parse_created_at(payload)?;
    let source_type = parse_source_type(payload)?;
    let rand = random_alnum6();
    let filename = match source_basename {
        Some(name) => format!("{created_at}-{rand}-{}", sanitize_basename(name)?),
        None => format!("{created_at}-{rand}.md"),
    };
    let common_path = format!("{project}/{theme}/{filename}");
    if !is_valid_common_path(&common_path) {
        return Err(json!({
            "error": format!("invalid common_path: {common_path}"),
            "_status": 400
        }));
    }
    Ok(CreateMeta {
        title,
        created_at,
        source_type,
        common_path,
    })
}

pub(super) fn body_from_source(document: &str) -> String {
    for sep in ["\n---\n", "\n---\r\n"] {
        if let Some(idx) = document.find(sep) {
            return document[idx + sep.len()..].trim().to_string();
        }
    }
    document.trim().to_string()
}

pub(super) fn assemble_raw(title: &str, created_at: &str, body: &str) -> Result<String, Value> {
    if body.trim().is_empty() {
        return Err(json!({ "error": "empty body after ---", "_status": 400 }));
    }
    let created_zh = format_created_at_zh(created_at).map_err(|e| {
        json!({ "error": e, "_status": 400 })
    })?;
    Ok(format!(
        "# {title}\n\n> 创建时间：{created_zh}\n\n---\n\n{body}\n"
    ))
}

pub(super) fn format_created_at_zh(ts: &str) -> Result<String, String> {
    if ts.len() != 12 || !ts.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid created_at: {ts}"));
    }
    let year: u32 = ts[0..4].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    let month: u32 = ts[4..6].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    let day: u32 = ts[6..8].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    let hour = &ts[8..10];
    let minute = &ts[10..12];
    Ok(format!("{year}年{month}月{day}日 {hour}:{minute}"))
}

fn parse_source_type(payload: &Value) -> Result<String, Value> {
    let raw = payload
        .get("source_type")
        .and_then(|v| v.as_str())
        .unwrap_or("summary")
        .trim();
    if SOURCE_TYPES.contains(&raw) {
        Ok(raw.to_string())
    } else {
        Err(json!({ "error": "Invalid source_type", "_status": 400 }))
    }
}

fn parse_created_at(payload: &Value) -> Result<String, Value> {
    match payload.get("created_at").and_then(|v| v.as_str()).map(str::trim) {
        None | Some("") => Ok(now_ts_utc8()),
        Some(ts) if ts.len() == 12 && ts.chars().all(|c| c.is_ascii_digit()) => {
            Ok(ts.to_string())
        }
        Some(_) => Err(json!({ "error": "Invalid created_at", "_status": 400 })),
    }
}

fn path_segment(payload: &Value, key: &str, default: &str) -> Result<String, Value> {
    let raw = payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default);
    if raw.contains("..") || raw.contains('/') || raw.contains('\\') {
        return Err(json!({ "error": format!("Invalid {key}"), "_status": 400 }));
    }
    Ok(raw.to_string())
}

fn sanitize_basename(name: &str) -> Result<String, Value> {
    let base = std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();
    if !base.to_ascii_lowercase().ends_with(".md") || base.contains("..") {
        return Err(json!({ "error": "Invalid source filename", "_status": 400 }));
    }
    let out: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if out == ".md" {
        return Err(json!({ "error": "Invalid source filename", "_status": 400 }));
    }
    Ok(out)
}

fn now_ts_utc8() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    Utc::now()
        .with_timezone(&tz)
        .format("%Y%m%d%H%M")
        .to_string()
}
