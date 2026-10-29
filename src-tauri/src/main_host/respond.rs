use serde_json::Value;
use tiny_http::{Header, Response, StatusCode};

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

pub(super) fn respond_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    respond_raw(request, status, body);
}

pub(super) fn respond_read_later_from_value(request: tiny_http::Request, value: Value) {
    let (status, body) = map_value_to_response(value);
    respond_with_cors(request, status, body);
}

pub(super) fn respond_read_later_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    respond_with_cors(request, status, body);
}

pub(super) fn cors_headers() -> [Header; 3] {
    [
        Header::from_bytes(b"Access-Control-Allow-Origin", b"*").expect("cors origin"),
        Header::from_bytes(b"Access-Control-Allow-Methods", b"POST, OPTIONS").expect("cors methods"),
        Header::from_bytes(b"Access-Control-Allow-Headers", b"Content-Type").expect("cors headers"),
    ]
}

pub(super) fn respond_with_cors(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    for header in cors_headers() {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

pub(super) fn respond_with_cors_empty(request: tiny_http::Request, status: u16) {
    let mut response = Response::from_string("").with_status_code(StatusCode(status));
    for header in cors_headers() {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

pub(super) fn respond_raw(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

pub(super) fn respond_html(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]) {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}
