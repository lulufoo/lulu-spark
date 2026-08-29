use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::workbench_read::get_notes_files_by_ids;

use super::respond::{respond_from_value, respond_json};

pub(super) fn handle_notes_files_post(repo_root: &PathBuf, mut request: tiny_http::Request) {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond_json(request, 400, json!({ "error": "Failed to read body" }));
        return;
    }
    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            respond_json(request, 400, json!({ "error": format!("Invalid JSON: {e}") }));
            return;
        }
    };
    let Some(ids_val) = payload.get("ids").and_then(|v| v.as_array()) else {
        respond_json(request, 400, json!({ "error": "Missing ids array" }));
        return;
    };
    let ids: Vec<String> = ids_val
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let value = get_notes_files_by_ids(repo_root, &ids);
    if value.get("_status").is_some() {
        respond_from_value(request, value);
        return;
    }
    respond_json(request, 200, value);
}

pub(super) fn handle_archive_post(
    repo_root: &PathBuf,
    mut request: tiny_http::Request,
    handler: fn(&Path, &Value) -> Value,
) {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond_json(request, 400, json!({ "error": "Failed to read body" }));
        return;
    }
    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            respond_json(request, 400, json!({ "error": format!("Invalid JSON: {e}") }));
            return;
        }
    };
    let value = handler(repo_root, &payload);
    respond_from_value(request, value);
}
