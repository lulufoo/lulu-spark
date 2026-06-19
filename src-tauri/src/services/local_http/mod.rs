//! Localhost read-only HTTP API for MCP sidecar proxy (`GET /api/corpus-*`, `/api/status`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use serde_json::{json, Value};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::services::workbench_read::{get_corpus_file, get_corpus_index};

pub struct LocalHttpHandle {
    server: Arc<Server>,
    join: JoinHandle<()>,
}

#[derive(Debug)]
pub enum LocalHttpError {
    BindFailed(String),
}

pub fn start(repo_root: PathBuf, port: u16) -> Result<LocalHttpHandle, LocalHttpError> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).map_err(|e| LocalHttpError::BindFailed(e.to_string()))?;
    let server = Arc::new(server);
    let server_for_thread = Arc::clone(&server);
    let join = thread::spawn(move || {
        for request in server_for_thread.incoming_requests() {
            handle_request(&repo_root, port, request);
        }
    });
    Ok(LocalHttpHandle { server, join })
}

pub fn stop(handle: LocalHttpHandle) {
    handle.server.unblock();
    let _ = handle.join.join();
}

pub(crate) fn map_value_to_response(value: Value) -> (u16, String) {
    let status = value
        .get("_status")
        .and_then(|v| v.as_u64())
        .unwrap_or(200) as u16;
    let mut body = value;
    if let Some(obj) = body.as_object_mut() {
        obj.remove("_status");
    }
    let json = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    (status, json)
}

fn handle_request(repo_root: &PathBuf, port: u16, request: tiny_http::Request) {
    if request.method() != &Method::Get {
        respond_json(
            request,
            405,
            json!({ "error": "Method not allowed" }),
        );
        return;
    }

    let path = request.url().split('?').next().unwrap_or("");
    match path {
        "/api/corpus-index" => {
            let value = get_corpus_index(repo_root);
            respond_from_value(request, value);
        }
        "/api/corpus-file" => {
            let params = parse_query(request.url());
            let layer = params.get("layer").map(String::as_str).unwrap_or("");
            let common_path = params.get("path").map(String::as_str).unwrap_or("");
            if layer != "digest" {
                respond_json(
                    request,
                    400,
                    json!({ "error": format!("Invalid layer: {layer}") }),
                );
                return;
            }
            let value = get_corpus_file(repo_root, layer, common_path);
            respond_from_value(request, value);
        }
        "/api/status" => {
            respond_json(
                request,
                200,
                json!({
                    "ok": true,
                    "http_port": port,
                }),
            );
        }
        _ => respond_json(request, 404, json!({ "error": "Not found" })),
    }
}

fn parse_query(url: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(query) = url.split('?').nth(1) else {
        return out;
    };
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("").to_string();
        let value = parts.next().unwrap_or("").to_string();
        if !key.is_empty() {
            out.insert(
                urlencoding::decode(&key)
                    .map(|s| s.into_owned())
                    .unwrap_or(key),
                urlencoding::decode(&value)
                    .map(|s| s.into_owned())
                    .unwrap_or(value),
            );
        }
    }
    out
}

fn respond_from_value(request: tiny_http::Request, value: Value) {
    let (status, body) = map_value_to_response(value);
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

fn respond_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

#[cfg(test)]
#[path = "../../unit-tests/services/local_http.rs"]
mod tests;
