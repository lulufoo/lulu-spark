use serde_json::Value;
use tauri::AppHandle;

use crate::config::paths;
use crate::services::{annotation, docs, doc_highlights, entry_write, tag_write};

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn set_done(_app: AppHandle, id: String, done: bool) -> Result<Value, String> {
    Ok(annotation::set_done(&repo_root()?, &id, done))
}

#[tauri::command]
pub fn set_importance(
    _app: AppHandle,
    id: String,
    importance: Option<String>,
) -> Result<Value, String> {
    Ok(annotation::set_importance(
        &repo_root()?,
        &id,
        importance,
    ))
}

#[tauri::command]
pub fn set_doc_source_type(
    _app: AppHandle,
    id: String,
    source_type: String,
) -> Result<Value, String> {
    Ok(docs::set_source_type(&id, &source_type))
}

#[tauri::command]
pub fn update_links(
    _app: AppHandle,
    id: String,
    links: Value,
) -> Result<Value, String> {
    Ok(annotation::update_links(&repo_root()?, &id, links))
}

#[tauri::command]
pub fn update_comments(
    _app: AppHandle,
    id: String,
    layer: String,
    comment: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(annotation::update_comments(
        &repo_root()?,
        &id,
        &layer,
        comment,
        ts,
    ))
}

#[tauri::command]
pub fn reorder_comments(
    _app: AppHandle,
    id: String,
    layer: String,
    ids: Vec<String>,
) -> Result<Value, String> {
    Ok(annotation::reorder_comments(
        &repo_root()?,
        &id,
        &layer,
        ids,
    ))
}

#[tauri::command]
pub fn update_highlights(
    _app: AppHandle,
    id: String,
    layer: String,
    highlight: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(annotation::update_highlights(
        &repo_root()?,
        &id,
        &layer,
        highlight,
        ts,
    ))
}

#[tauri::command]
pub fn update_doc_highlights(
    _app: AppHandle,
    key: String,
    highlight: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(doc_highlights::update_doc_highlights(&key, highlight, &ts))
}

#[tauri::command]
pub fn save_entry(
    _app: AppHandle,
    layer: String,
    id: String,
    content: String,
    lang: Option<String>,
) -> Result<Value, String> {
    Ok(entry_write::save_entry(
        &repo_root()?,
        layer,
        id,
        content,
        lang,
    ))
}

#[tauri::command]
pub fn write_abs_file(_app: AppHandle, path: String, content: String) -> Result<Value, String> {
    Ok(crate::services::abs_file::write_abs_file(&path, &content))
}

#[tauri::command]
pub fn tag_attach(
    _app: AppHandle,
    id: String,
    key: Option<String>,
    value: Option<String>,
) -> Result<Value, String> {
    let mut payload = serde_json::Map::new();
    if let Some(k) = key {
        payload.insert("key".into(), Value::String(k));
    }
    if let Some(v) = value {
        payload.insert("value".into(), Value::String(v));
    }
    Ok(tag_write::tag_attach(
        &repo_root()?,
        &id,
        &Value::Object(payload),
    ))
}

#[tauri::command]
pub fn tag_detach(
    _app: AppHandle,
    id: String,
    key: String,
) -> Result<Value, String> {
    Ok(tag_write::tag_detach(&repo_root()?, &id, &key))
}

#[tauri::command]
pub fn tag_update_value(
    _app: AppHandle,
    key: String,
    value: String,
) -> Result<Value, String> {
    Ok(tag_write::tag_update_value(&repo_root()?, &key, &value))
}

/// Thin Tauri wrapper around the docs service: `workspace_id` + `project_id` in the payload locate
/// the new document (default workspace, inbox when `project_id` is omitted).
/// Business errors stay in the Value (`error` + `_status`); do not convert to Err.
/// App jot-create may pass `{ body, source_type: "jot" }` (no `source_path`) → Host synthesis.
/// Path create uses `source_path` + `title`; `document` is rejected.
pub fn create_note_json(payload: Value) -> Result<Value, String> {
    if payload.get("document").is_some() {
        return Ok(serde_json::json!({
            "error": "document is not supported; use source_path",
            "_status": 400
        }));
    }
    if let Some(body) = payload.get("body").and_then(|v| v.as_str()) {
        let source_type = payload
            .get("source_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if source_type == "jot" {
            let workspace = payload.get("workspace_id").and_then(|v| v.as_str());
            let project = payload.get("project_id").and_then(|v| v.as_str());
            return match docs::create_jot(workspace, project, body, &docs::JotCreateOpts::default()) {
                Ok(v) => Ok(v),
                Err(msg) => {
                    let (status, error) = match msg.split_once(':') {
                        Some((s, e)) => (
                            s.trim().parse::<u64>().unwrap_or(500),
                            e.trim().to_string(),
                        ),
                        None => (500u64, msg),
                    };
                    Ok(serde_json::json!({ "error": error, "_status": status }))
                }
            };
        }
    }
    Ok(if payload.get("source_path").is_some() {
        docs::create_doc(&payload)
    } else {
        docs::create_doc_content(&payload)
    })
}

#[tauri::command]
pub fn create_note(_app: AppHandle, payload: Value) -> Result<Value, String> {
    create_note_json(payload)
}

pub fn mark_message_channel_read_json(channel: &str) -> Result<(), String> {
    crate::services::message_center::mark_channel_read(channel)
}

#[tauri::command]
pub fn mark_message_channel_read(_app: AppHandle, channel: String) -> Result<(), String> {
    mark_message_channel_read_json(&channel)
}

#[cfg(test)]
#[path = "../unit-tests/commands/create_note_write.rs"]
mod create_note_write_tests;

#[cfg(test)]
#[path = "../unit-tests/commands/message_center.rs"]
mod message_center_tests;
