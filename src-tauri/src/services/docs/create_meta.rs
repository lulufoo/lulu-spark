//! Shared create-note metadata: title, path segments, source_type.

use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};

use crate::services::archive_parse::is_valid_common_path;
use crate::services::id::random_alnum6;

pub(crate) struct CreateMeta {
    pub title: String,
    pub created_at: String,
    pub source_type: String,
    /// Project id from the payload; `None` means the workspace inbox.
    pub project: Option<String>,
    /// Stored rel: `{folder}/{filename}` — no project folder prefix.
    pub common_path: String,
}

/// Title, path, `created_at` and `source_type` checks. The caller resolves the project
/// itself (per workspace).
pub(crate) fn parse_create_meta_unchecked(
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

    let project = optional_project(payload)?;
    if payload.get("theme").is_some() {
        return Err(json!({ "error": "theme was renamed to folder", "_status": 400 }));
    }
    let folder = folder_path(payload)?;
    let created_at = parse_created_at(payload)?;
    let source_type = parse_source_type(payload)?;
    let rand = random_alnum6();
    let filename = match source_basename {
        Some(name) => format!("{created_at}-{rand}-{}", sanitize_basename(name)?),
        None => format!("{created_at}-{rand}.md"),
    };
    let common_path = format!("{folder}/{filename}");
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
        project,
        common_path,
    })
}

pub(crate) fn format_created_at_zh(ts: &str) -> Result<String, String> {
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
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(raw) = raw else {
        return Err(json!({ "error": "Missing source_type", "_status": 400 }));
    };
    if super::categories::is_legal(raw) {
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

fn optional_project(payload: &Value) -> Result<Option<String>, Value> {
    match payload.get("project") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => {
            let id = raw.trim();
            if id.is_empty() {
                Ok(None)
            } else if id.contains("..") || id.contains('/') || id.contains('\\') {
                Err(json!({ "error": "Invalid project", "_status": 400 }))
            } else {
                Ok(Some(id.to_string()))
            }
        }
        Some(_) => Err(json!({ "error": "Invalid project", "_status": 400 })),
    }
}

/// Deepest `folder` allowed under a project.
pub(crate) const MAX_FOLDER_DEPTH: usize = 3;

/// `folder`: a relative path of 1..=3 segments (`a`, `a/b`, `a/b/c`); defaults to `notes`.
/// Empty segments, leading or trailing `/`, `.`/`..` and `\` are rejected.
fn folder_path(payload: &Value) -> Result<String, Value> {
    let raw = payload
        .get("folder")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("notes");
    let segments: Vec<&str> = raw.split('/').collect();
    let ok = segments.len() <= MAX_FOLDER_DEPTH
        && !raw.contains('\\')
        && segments
            .iter()
            .all(|s| !s.is_empty() && *s != "." && !s.contains(".."));
    if !ok {
        return Err(json!({
            "error": format!("Invalid folder: use 1 to {MAX_FOLDER_DEPTH} path segments such as a/b/c"),
            "_status": 400
        }));
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
