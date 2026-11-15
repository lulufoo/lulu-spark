//! Board source validation and Sketch-only projection.

use base64::Engine;
use serde_json::Value;

use super::{failure as reply, BoardSketchReply};

/// A validated, type-independent source. URL and diagram ID remain with the
/// caller's validated request; neither is replaced by response metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoardDocument {
    pub bmd: String,
    pub title: String,
    pub version: i32,
}

/// Validate source/metadata before choosing a renderer. A malformed Sketch
/// projection does not turn an otherwise valid Board document into bad cache.
pub(crate) fn parse_document(body: &str) -> Result<BoardDocument, BoardSketchReply> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Err(reply("invalid_document"));
    };
    let Some(bmd) = value.get("bmd").and_then(Value::as_str) else {
        return Err(reply("invalid_document"));
    };
    let Some(version) = value
        .get("version")
        .and_then(Value::as_i64)
        .filter(|version| (1..=i64::from(i32::MAX)).contains(version))
    else {
        return Err(reply("invalid_document"));
    };
    let title = match value.get("title") {
        Some(Value::String(title)) => title.clone(),
        None => String::new(),
        _ => return Err(reply("invalid_document")),
    };
    Ok(BoardDocument {
        bmd: bmd.to_string(),
        title,
        version: version as i32,
    })
}

pub(crate) fn project_sketch(document: &BoardDocument) -> BoardSketchReply {
    let mut projection = BoardSketchReply {
        status: "not_sketch",
        content_b64: None,
        title: diagram_title(&document.title),
        version: Some(document.version),
        invalidation_scope: None,
        identity: None,
    };
    if !body_is_sketch(&document.bmd) {
        return projection;
    }
    let Some(token) = content_token(&document.bmd) else {
        projection.status = "invalid_source";
        return projection;
    };
    if token == document.bmd.trim() || !is_base64(token) {
        projection.status = "invalid_source";
        return projection;
    }
    projection.status = "sketch";
    projection.content_b64 = Some(token.to_string());
    projection
}

fn diagram_title(title: &str) -> Option<String> {
    let title = title.trim();
    if title.is_empty() { None } else { Some(title.to_string()) }
}

fn body_is_sketch(body: &str) -> bool {
    let Some(line) = kind_line(body) else {
        return false;
    };
    let lower = line.to_ascii_lowercase();
    lower == "sketch" || lower.strip_prefix("sketch").is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn kind_line(body: &str) -> Option<&str> {
    for line in body.split('\n') {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("//")
            || line.starts_with("%%")
            || line.starts_with("meta ")
            || line.starts_with("---")
            || is_style_line(line)
        {
            continue;
        }
        return Some(line);
    }
    None
}

fn is_style_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix("%%")
        .map(str::trim_start)
        .unwrap_or(lower.as_str());
    let Some(after) = rest.strip_prefix("style") else {
        return false;
    };
    if !after.starts_with(char::is_whitespace) {
        return false;
    }
    let token = after.trim();
    !token.is_empty() && !token.chars().any(char::is_whitespace)
}

fn content_token(body: &str) -> Option<&str> {
    for line in body.split('\n') {
        let line = line.trim();
        let lower = line.to_ascii_lowercase();
        let Some(after) = lower.strip_prefix("content") else {
            continue;
        };
        if !after.starts_with(char::is_whitespace) {
            continue;
        }
        let token = line["content".len()..].trim();
        if token.is_empty() || token.chars().any(char::is_whitespace) {
            continue;
        }
        return Some(token);
    }
    None
}

fn is_base64(token: &str) -> bool {
    base64::engine::general_purpose::STANDARD
        .decode(token)
        .is_ok()
}


#[cfg(test)]
#[path = "../../unit-tests/services/board_sketch/source.rs"]
mod tests;
