use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::workbench_read::{
    get_note_content_by_id, get_note_digest_by_id, list_notes_by_catalog,
};

use super::respond::{respond_from_value, respond_json};

fn read_json_body(request: &mut tiny_http::Request) -> Result<Value, Value> {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Err(json!({ "error": "Failed to read body", "_status": 400 }));
    }
    serde_json::from_str(&body).map_err(|e| json!({ "error": format!("Invalid JSON: {e}"), "_status": 400 }))
}

fn respond_read(request: tiny_http::Request, value: Value) {
    if value.get("_status").is_some() {
        respond_from_value(request, value);
        return;
    }
    respond_json(request, 200, value);
}

pub(super) fn handle_notes_by_catalog_post(repo_root: &PathBuf, mut request: tiny_http::Request) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_from_value(request, err);
            return;
        }
    };
    let Some(catalog) = payload.get("catalog").and_then(|v| v.as_str()) else {
        respond_json(request, 400, json!({ "error": "Missing catalog" }));
        return;
    };
    respond_read(request, list_notes_by_catalog(repo_root, catalog));
}

pub(super) fn handle_note_digest_post(repo_root: &PathBuf, mut request: tiny_http::Request) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_from_value(request, err);
            return;
        }
    };
    let Some(id) = payload.get("id").and_then(|v| v.as_str()) else {
        respond_json(request, 400, json!({ "error": "Missing id" }));
        return;
    };
    respond_read(request, get_note_digest_by_id(repo_root, id));
}

pub(super) fn handle_note_content_post(repo_root: &PathBuf, mut request: tiny_http::Request) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_from_value(request, err);
            return;
        }
    };
    let Some(id) = payload.get("id").and_then(|v| v.as_str()) else {
        respond_json(request, 400, json!({ "error": "Missing id" }));
        return;
    };
    respond_read(request, get_note_content_by_id(repo_root, id));
}

pub(super) fn handle_archive_post(
    repo_root: &PathBuf,
    mut request: tiny_http::Request,
    handler: fn(&Path, &Value) -> Value,
) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_from_value(request, err);
            return;
        }
    };
    let value = handler(repo_root, &payload);
    respond_from_value(request, value);
}
