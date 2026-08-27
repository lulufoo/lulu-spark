use serde_json::json;

use crate::services::bind::{complete_bind, BindError};

use super::respond::respond_json;

pub(super) fn handle_bind_complete(mut request: tiny_http::Request) {
    let mut body = Vec::new();
    if request.as_reader().read_to_end(&mut body).is_err() {
        respond_json(request, 400, json!({ "error": "invalid_request" }));
        return;
    }
    match complete_bind(&body) {
        Ok(result) => respond_json(
            request,
            200,
            json!({
                "device_mcp_token": result.device_mcp_token,
                "binding_public_key": result.binding_public_key,
            }),
        ),
        Err(err) => {
            let status = match err {
                BindError::keychain_unavailable => 500,
                _ => 400,
            };
            respond_json(request, status, json!({ "error": format!("{err:?}") }));
        }
    }
}
