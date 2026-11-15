//! Board transport and reply contract. Credentials stay in the Host coordinator.

use std::time::Duration;
use serde::Serialize;
use crate::services::board_api;

mod source;
pub(crate) use source::{parse_document, project_sketch, BoardDocument};

const BOARD_ORIGIN: &str = "luluboard.app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardInvalidationScope { Credential, Diagram }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoardReadIdentity {
    pub credential_scope: String,
    pub generation: u64,
    pub diagram_id: String,
    pub diagram_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoardSketchReply {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_b64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidation_scope: Option<BoardInvalidationScope>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub identity: Option<BoardReadIdentity>,
}

pub fn read_board_sketch(url: &str) -> BoardSketchReply {
    crate::services::board_cache::read_board_sketch(url)
}

pub(crate) fn log_read(
    reply: &BoardSketchReply, diagram_id: &str, total: Duration,
    vault: Option<Duration>, fetch: Option<Duration>, bytes: Option<usize>,
) {
    crate::services::board_sketch_log::read(reply.status, diagram_id, total, vault, fetch, bytes);
}

pub(crate) fn failure(status: &'static str) -> BoardSketchReply {
    BoardSketchReply {
        status, content_b64: None, title: None, version: None, identity: None,
        invalidation_scope: match status {
            "unconfigured" | "unauthorized" => Some(BoardInvalidationScope::Credential),
            "forbidden" | "not_found" => Some(BoardInvalidationScope::Diagram),
            _ => None,
        },
    }
}

pub fn board_graph_request_url(diagram_id: &str) -> Option<String> {
    is_graph_id(diagram_id).then(|| format!("{}?diagram_id={diagram_id}", board_api::GRAPH_URL))
}

pub(crate) fn diagram_id(raw: &str) -> Result<String, &'static str> {
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


pub(crate) struct DocumentFetch {
    pub result: Result<BoardDocument, BoardSketchReply>,
    pub bytes: Option<usize>,
}

pub(crate) fn read_http(status: u16, body: &str) -> Result<BoardDocument, BoardSketchReply> {
    match status {
        200 => parse_document(body),
        401 => Err(failure("unauthorized")),
        403 => Err(failure("forbidden")),
        404 => Err(failure("not_found")),
        _ => Err(failure("network")),
    }
}

pub(crate) fn fetch_document(pat: &str, diagram_id: &str) -> DocumentFetch {
    let network = || DocumentFetch { result: Err(failure("network")), bytes: None };
    let Some(url) = board_graph_request_url(diagram_id) else { return network(); };
    let Some(client) = board_api::client() else { return network(); };
    let Ok(response) = client.get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {pat}"))
        .header(reqwest::header::ACCEPT, "application/json").send()
    else { return network(); };
    let status = response.status().as_u16();
    match response.text() {
        Ok(body) => DocumentFetch { bytes: Some(body.len()), result: read_http(status, &body) },
        Err(_) => network(),
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/board_sketch.rs"]
mod tests;
