use crate::services::notes::{
    create_notes_category_value, delete_notes_category_value, list_notes_categories_value,
    update_notes_category_value,
};

use super::respond::{read_json_body, respond_from_value, respond_json};

pub(super) fn handle_notes_categories_get(request: tiny_http::Request) {
    respond_from_value(request, list_notes_categories_value());
}

pub(super) fn handle_notes_category_create(mut request: tiny_http::Request) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_json(request, 400, err);
            return;
        }
    };
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    respond_from_value(
        request,
        create_notes_category_value(id, title, description),
    );
}

pub(super) fn handle_notes_category_update(mut request: tiny_http::Request) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_json(request, 400, err);
            return;
        }
    };
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    respond_from_value(
        request,
        update_notes_category_value(id, title, description),
    );
}

pub(super) fn handle_notes_category_delete(mut request: tiny_http::Request) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_json(request, 400, err);
            return;
        }
    };
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    respond_from_value(request, delete_notes_category_value(id));
}
