use serde_json::json;
use tiny_http::Method;

use super::auth_login::handle_auth_login_landing;
use super::bind::handle_bind_complete;
use super::read_later::handle_read_later_post;
use super::respond::{respond_json, respond_with_cors_empty};

pub(super) fn is_read_later_path(path: &str) -> bool {
    path == "/api/read-later" || path.starts_with("/api/read-later/")
}

pub(super) fn handle_request(request: tiny_http::Request) {
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();

    if request.method() == &Method::Options {
        if is_read_later_path(&path) {
            respond_with_cors_empty(request, 204);
            return;
        }
        respond_json(request, 405, json!({ "error": "Method not allowed" }));
        return;
    }

    if request.method() == &Method::Post {
        match path.as_str() {
            "/api/read-later" => {
                handle_read_later_post(request);
                return;
            }
            "/api/bind-complete" => {
                handle_bind_complete(request);
                return;
            }
            _ => {}
        }
    }

    if request.method() == &Method::Get {
        if path == "/api/auth-login/landing" {
            handle_auth_login_landing(request);
            return;
        }
        respond_json(request, 404, json!({ "error": "Not found" }));
        return;
    }

    respond_json(request, 405, json!({ "error": "Method not allowed" }));
}
