//! Read one Board Sketch from its public page URL.
//!
//! The request always goes to the Board graph API. The page URL is never the
//! request target, and the PAT is never copied into the result.

use std::time::{Duration, Instant};

use base64::Engine;
use serde::Serialize;
use serde_json::Value;

use crate::config::vault;
use crate::services::board_api;

const BOARD_ORIGIN: &str = "luluboard.app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardInvalidationScope {
    Credential,
    Diagram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoardSketchReply {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_b64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Present for a valid source document, even if its preview is unsupported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidation_scope: Option<BoardInvalidationScope>,
}

/// A validated, type-independent source. URL and diagram ID remain with the
/// caller's validated request; neither is replaced by response metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoardDocument {
    pub bmd: String,
    pub title: String,
    pub version: i32,
}

enum FetchOutcome {
    Http { status: u16, body: String },
    Network,
}

pub fn read_board_sketch(url: &str) -> BoardSketchReply {
    read_board_sketch_with(url, fetch_graph)
}

fn read_board_sketch_with<F>(url: &str, fetch: F) -> BoardSketchReply
where
    F: FnOnce(&str, &str) -> FetchOutcome,
{
    let started = Instant::now();
    let diagram_id = match diagram_id(url) {
        Ok(id) => id,
        Err(status) => return logged(reply(status), "", started, None, None, None),
    };
    let vault_at = Instant::now();
    let pat = match vault::get_board_pat() {
        Ok(Some(pat)) => pat,
        Ok(None) => {
            return logged(reply("unconfigured"), &diagram_id, started, Some(vault_at.elapsed()), None, None);
        }
        Err(_) => {
            return logged(reply("unavailable"), &diagram_id, started, Some(vault_at.elapsed()), None, None);
        }
    };
    let vault_ms = Some(vault_at.elapsed());
    let fetch_at = Instant::now();
    match fetch(&pat, &diagram_id) {
        FetchOutcome::Network => {
            logged(reply("network"), &diagram_id, started, vault_ms, Some(fetch_at.elapsed()), None)
        }
        FetchOutcome::Http { status, body } => {
            let bytes = body.len();
            logged(
                interpret(status, &body),
                &diagram_id,
                started,
                vault_ms,
                Some(fetch_at.elapsed()),
                Some(bytes),
            )
        }
    }
}

fn logged(
    reply: BoardSketchReply,
    diagram_id: &str,
    started: Instant,
    vault_ms: Option<Duration>,
    fetch_ms: Option<Duration>,
    bytes: Option<usize>,
) -> BoardSketchReply {
    crate::services::board_sketch_log::read(reply.status, diagram_id, started.elapsed(), vault_ms, fetch_ms, bytes);
    reply
}

fn reply(status: &'static str) -> BoardSketchReply {
    BoardSketchReply {
        status,
        content_b64: None,
        title: None,
        version: None,
        invalidation_scope: match status {
            "unconfigured" | "unauthorized" => Some(BoardInvalidationScope::Credential),
            "forbidden" | "not_found" => Some(BoardInvalidationScope::Diagram),
            _ => None,
        },
    }
}

pub fn board_graph_request_url(diagram_id: &str) -> Option<String> {
    if !is_graph_id(diagram_id) {
        return None;
    }
    Some(format!("{}?diagram_id={diagram_id}", board_api::GRAPH_URL))
}

fn diagram_id(raw: &str) -> Result<String, &'static str> {
    let raw = raw.trim();
    let Some(rest) = raw.strip_prefix("https://") else {
        return Err("not_board");
    };
    let (before_hash, fragment) = match rest.split_once('#') {
        Some((head, frag)) => (head, frag),
        None => (rest, ""),
    };
    if !is_board_origin(before_hash) {
        return Err("not_board");
    }
    graph_id_from_fragment(fragment).ok_or("invalid_id")
}

fn is_board_origin(before_hash: &str) -> bool {
    let authority = before_hash.split(['/', '?']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
            (host, Some(port))
        }
        _ => (authority, None),
    };
    if !host.eq_ignore_ascii_case(BOARD_ORIGIN) {
        return false;
    }
    matches!(port, None | Some("443"))
}

fn graph_id_from_fragment(fragment: &str) -> Option<String> {
    let rest = fragment.strip_prefix("b:")?;
    let token: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit() || *ch == '_')
        .collect();
    let boundary = rest[token.len()..].chars().next();
    if is_graph_id(&token) && boundary.is_none_or(is_id_boundary) {
        Some(token)
    } else {
        None
    }
}

fn is_id_boundary(ch: char) -> bool {
    matches!(ch, '&' | '?' | '/' | ' ')
}

fn is_graph_id(id: &str) -> bool {
    let Some(hex) = id.strip_prefix("b_") else {
        return false;
    };
    hex.len() == 8 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

fn interpret(status: u16, body: &str) -> BoardSketchReply {
    if (300..400).contains(&status) {
        return reply("network");
    }
    match status {
        401 => reply("unauthorized"),
        403 => reply("forbidden"),
        404 => reply("not_found"),
        200 => match parse_document(body) {
            Ok(document) => project_sketch(&document),
            Err(failure) => failure,
        },
        _ => reply("network"),
    }
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

fn fetch_graph(pat: &str, diagram_id: &str) -> FetchOutcome {
    let Some(url) = board_graph_request_url(diagram_id) else {
        return FetchOutcome::Network;
    };
    let Some(client) = board_api::client() else {
        return FetchOutcome::Network;
    };
    let response = client
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {pat}"))
        .header(reqwest::header::ACCEPT, "application/json")
        .send();
    let Ok(response) = response else {
        return FetchOutcome::Network;
    };
    let status = response.status().as_u16();
    match response.text() {
        Ok(body) => FetchOutcome::Http { status, body },
        Err(_) => FetchOutcome::Network,
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/board_sketch.rs"]
mod tests;
