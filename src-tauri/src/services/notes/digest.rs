use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::create_meta::body_from_source;
use super::store::{notes_layer_path, write_markdown_atomic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DigestMode {
    Auto,
    Always,
    Never,
}

pub(super) fn parse_digest_mode(payload: &Value) -> Result<DigestMode, Value> {
    match payload.get("digest").and_then(|v| v.as_str()).map(str::trim) {
        Some("auto") => Ok(DigestMode::Auto),
        Some("always") => Ok(DigestMode::Always),
        Some("never") => Ok(DigestMode::Never),
        Some(_) => Err(json!({ "error": "Invalid digest", "_status": 400 })),
        None => Err(json!({ "error": "Missing digest", "_status": 400 })),
    }
}

pub(super) fn digest_body_from_payload(payload: &Value) -> Option<String> {
    payload
        .get("digest_body")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// `auto` follows Host AD-0; `always` writes; `never` skips.
pub(super) fn digest_should_write(mode: DigestMode, raw: &str, source_type: &str) -> bool {
    match mode {
        DigestMode::Always => true,
        DigestMode::Never => false,
        DigestMode::Auto => digest_applies(raw, source_type),
    }
}

/// AD-0: digest is worth writing when any rule matches.
pub(super) fn digest_applies(raw: &str, source_type: &str) -> bool {
    if turn_block_count(raw) >= 2 {
        return true;
    }
    if theme_heading_count(raw) >= 2 {
        return true;
    }
    let body_len = body_from_source(raw).chars().count();
    if body_len >= 600 {
        return true;
    }
    if source_type == "summary" && body_len >= 200 {
        return true;
    }
    if source_type == "article" && body_len >= 600 {
        return true;
    }
    false
}

pub(super) fn write_digest_file(
    notes: &Path,
    common_path: &str,
    digest_body: &str,
    written: &mut Vec<PathBuf>,
) -> Result<String, Value> {
    let digest_path = notes_layer_path(notes, "digest", common_path)?;
    if digest_path.is_file() {
        return Err(json!({
            "error": format!("File already exists: digest/{common_path}"),
            "_status": 409
        }));
    }
    if let Err(e) = write_markdown_atomic(&digest_path, digest_body) {
        return Err(json!({ "error": e, "_status": 500 }));
    }
    written.push(digest_path);
    Ok(format!("digest/{common_path}"))
}

/// Overwrite digest if it already exists (update_note). Create-note stays on
/// [`write_digest_file`] and still 409s when the digest file is present.
pub(super) fn write_digest_file_overwrite(
    notes: &Path,
    common_path: &str,
    digest_body: &str,
) -> Result<String, Value> {
    let digest_path = notes_layer_path(notes, "digest", common_path)?;
    if let Err(e) = write_markdown_atomic(&digest_path, digest_body) {
        return Err(json!({ "error": e, "_status": 500 }));
    }
    Ok(format!("digest/{common_path}"))
}

fn turn_block_count(raw: &str) -> usize {
    let headings = raw
        .lines()
        .filter(|line| {
            let t = line.trim();
            t.starts_with("## Turn ") || t.starts_with("## TURN ")
        })
        .count();
    let seps = raw.matches("<!-- DDM:TURN_SEP:v1 -->").count();
    headings.max(seps)
}

pub(super) fn digest_body_rejects_relative_images(digest_body: &str) -> Option<Value> {
    if has_relative_image(digest_body) {
        Some(json!({
            "error": "digest_body must not contain relative images",
            "_status": 400
        }))
    } else {
        None
    }
}

fn has_relative_image(md: &str) -> bool {
    markdown_images_have_relative(md) || html_img_has_relative(md)
}

fn href_is_relative(raw: &str) -> bool {
    let raw = raw.trim();
    let raw = raw
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(raw);
    let href = raw
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c| c == '"' || c == '\'');
    if href.is_empty() || href.starts_with('#') {
        return false;
    }
    let lower = href.to_ascii_lowercase();
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || href.starts_with('/')
    {
        return false;
    }
    true
}

fn markdown_images_have_relative(md: &str) -> bool {
    let mut rest = md;
    while let Some(idx) = rest.find("![") {
        let after = &rest[idx + 2..];
        let Some(close) = after.find("](") else {
            rest = &after[1.min(after.len())..];
            continue;
        };
        let href_part = &after[close + 2..];
        let Some(end) = href_part.find(')') else {
            break;
        };
        if href_is_relative(&href_part[..end]) {
            return true;
        }
        rest = &href_part[end + 1..];
    }
    false
}

fn html_img_has_relative(md: &str) -> bool {
    let lower = md.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(idx) = lower[search..].find("<img") {
        let abs = search + idx;
        let Some(tag_end) = md[abs..].find('>') else {
            break;
        };
        let tag = &md[abs..abs + tag_end];
        if let Some(href) = html_src_value(tag) {
            if href_is_relative(&href) {
                return true;
            }
        }
        search = abs + 4;
    }
    false
}

fn html_src_value(tag: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let idx = lower.find("src=")?;
    let after = tag[idx + 4..].trim_start();
    let first = after.chars().next()?;
    if first == '"' || first == '\'' {
        let rest = &after[first.len_utf8()..];
        let end = rest.find(first)?;
        return Some(rest[..end].to_string());
    }
    let end = after
        .find(|c: char| c.is_whitespace() || c == '>')
        .unwrap_or(after.len());
    Some(after[..end].to_string())
}

fn theme_heading_count(raw: &str) -> usize {
    raw.lines()
        .filter(|line| {
            let t = line.trim();
            t.starts_with("## ") && !t.starts_with("## Turn ") && !t.starts_with("## TURN ")
        })
        .count()
}
