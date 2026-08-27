use serde_json::Value;
use tauri::AppHandle;

use crate::config::paths;
use crate::services::sediment_kb::{self, SedimentKbError};
use crate::services::{annotation, archive_write, doc_highlights, entry_write, kb_write, tag_write};

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

pub fn map_sediment_kb_error(err: SedimentKbError) -> Value {
    match err {
        SedimentKbError::InvalidFormat => serde_json::json!({
            "error": "无效的仓库地址，请使用 owner/repo 或 GitHub URL",
            "code": "invalid_format",
            "_status": 400,
        }),
        SedimentKbError::NotAccessible(msg) if msg.contains("GitHub Token") => serde_json::json!({
            "error": "需要配置 GitHub Token 才能验证仓库",
            "code": "no_token",
            "_status": 401,
        }),
        SedimentKbError::NotAccessible(_) => serde_json::json!({
            "error": "无法访问该仓库，请检查地址与 GitHub 权限",
            "code": "not_accessible",
            "_status": 403,
        }),
        SedimentKbError::Duplicate => serde_json::json!({
            "error": "该仓库已在沉淀知识库中",
            "code": "duplicate",
            "_status": 409,
        }),
        SedimentKbError::InvalidName => serde_json::json!({
            "error": "invalid category name",
            "code": "invalid_name",
            "_status": 400,
        }),
        SedimentKbError::ProtectedCategory => serde_json::json!({
            "error": "protected category",
            "code": "protected_category",
            "_status": 400,
        }),
        SedimentKbError::CategoryNotFound => serde_json::json!({
            "error": "category not found",
            "code": "category_not_found",
            "_status": 404,
        }),
        SedimentKbError::RepoNotFound => serde_json::json!({
            "error": "repo not found",
            "code": "repo_not_found",
            "_status": 404,
        }),
        SedimentKbError::Io(msg) => serde_json::json!({
            "error": msg,
            "code": "io_error",
            "_status": 500,
        }),
    }
}

fn sediment_kb_ok() -> Value {
    serde_json::json!({ "ok": true })
}

pub fn sediment_kb_add_repo_json(payload: Value) -> Result<Value, String> {
    let full_name = payload
        .get("full_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing full_name".to_string())?;
    let category_id = payload
        .get("category_id")
        .and_then(|v| v.as_str());
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match sediment_kb::add_repo(full_name, category_id, description) {
        Ok(()) => Ok(sediment_kb_ok()),
        Err(e) => Ok(map_sediment_kb_error(e)),
    }
}

pub fn sediment_kb_remove_repo_json(payload: Value) -> Result<Value, String> {
    let full_name = payload
        .get("full_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing full_name".to_string())?;
    match sediment_kb::remove_repo(full_name) {
        Ok(()) => Ok(sediment_kb_ok()),
        Err(e) => Ok(map_sediment_kb_error(e)),
    }
}

pub fn sediment_kb_update_repo_category_json(payload: Value) -> Result<Value, String> {
    let full_name = payload
        .get("full_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing full_name".to_string())?;
    let category_id = payload
        .get("category_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing category_id".to_string())?;
    match sediment_kb::update_repo_category(full_name, category_id) {
        Ok(()) => Ok(sediment_kb_ok()),
        Err(e) => Ok(map_sediment_kb_error(e)),
    }
}

pub fn sediment_kb_add_category_json(payload: Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing name".to_string())?;
    match sediment_kb::add_category(name) {
        Ok(id) => Ok(serde_json::json!({ "ok": true, "id": id })),
        Err(e) => Ok(map_sediment_kb_error(e)),
    }
}

pub fn sediment_kb_rename_category_json(payload: Value) -> Result<Value, String> {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing id".to_string())?;
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing name".to_string())?;
    match sediment_kb::rename_category(id, name) {
        Ok(()) => Ok(sediment_kb_ok()),
        Err(e) => Ok(map_sediment_kb_error(e)),
    }
}

pub fn sediment_kb_remove_category_json(payload: Value) -> Result<Value, String> {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing id".to_string())?;
    match sediment_kb::remove_category(id) {
        Ok(()) => Ok(sediment_kb_ok()),
        Err(e) => Ok(map_sediment_kb_error(e)),
    }
}

#[tauri::command]
pub fn sediment_kb_add_repo(_app: AppHandle, payload: Value) -> Result<Value, String> {
    sediment_kb_add_repo_json(payload)
}

#[tauri::command]
pub fn sediment_kb_remove_repo(_app: AppHandle, payload: Value) -> Result<Value, String> {
    sediment_kb_remove_repo_json(payload)
}

#[tauri::command]
pub fn sediment_kb_update_repo_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    sediment_kb_update_repo_category_json(payload)
}

#[tauri::command]
pub fn sediment_kb_add_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    sediment_kb_add_category_json(payload)
}

#[tauri::command]
pub fn sediment_kb_rename_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    sediment_kb_rename_category_json(payload)
}

#[tauri::command]
pub fn sediment_kb_remove_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    sediment_kb_remove_category_json(payload)
}

#[tauri::command]
pub fn set_done(_app: AppHandle, common_path: String, done: bool) -> Result<Value, String> {
    Ok(annotation::set_done(&repo_root()?, &common_path, done))
}

#[tauri::command]
pub fn set_importance(
    _app: AppHandle,
    common_path: String,
    importance: Option<String>,
) -> Result<Value, String> {
    Ok(annotation::set_importance(
        &repo_root()?,
        &common_path,
        importance,
    ))
}

#[tauri::command]
pub fn update_links(
    _app: AppHandle,
    common_path: String,
    links: Value,
) -> Result<Value, String> {
    Ok(annotation::update_links(&repo_root()?, &common_path, links))
}

#[tauri::command]
pub fn update_comments(
    _app: AppHandle,
    common_path: String,
    layer: String,
    comment: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(annotation::update_comments(
        &repo_root()?,
        &common_path,
        &layer,
        comment,
        ts,
    ))
}

#[tauri::command]
pub fn reorder_comments(
    _app: AppHandle,
    common_path: String,
    layer: String,
    ids: Vec<String>,
) -> Result<Value, String> {
    Ok(annotation::reorder_comments(
        &repo_root()?,
        &common_path,
        &layer,
        ids,
    ))
}

#[tauri::command]
pub fn update_highlights(
    _app: AppHandle,
    common_path: String,
    layer: String,
    highlight: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(annotation::update_highlights(
        &repo_root()?,
        &common_path,
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
    common_path: String,
    content: String,
) -> Result<Value, String> {
    Ok(entry_write::save_entry(
        &repo_root()?,
        layer,
        common_path,
        content,
    ))
}

#[tauri::command]
pub fn kb_save(
    _app: AppHandle,
    repo: String,
    path: String,
    content: String,
) -> Result<Value, String> {
    Ok(kb_write::kb_save(&repo_root()?, repo, path, content))
}

#[tauri::command]
pub fn kb_update_comments(
    _app: AppHandle,
    repo: String,
    path: String,
    comment: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(kb_write::kb_update_comments(
        &repo_root()?,
        repo,
        path,
        comment,
        ts,
    ))
}

#[tauri::command]
pub fn kb_reorder_comments(
    _app: AppHandle,
    repo: String,
    path: String,
    ids: Vec<String>,
) -> Result<Value, String> {
    Ok(kb_write::kb_reorder_comments(&repo_root()?, repo, path, ids))
}

#[tauri::command]
pub fn kb_update_highlights(
    _app: AppHandle,
    repo: String,
    path: String,
    highlight: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(kb_write::kb_update_highlights(
        &repo_root()?,
        repo,
        path,
        highlight,
        ts,
    ))
}

#[tauri::command]
pub fn kb_update_links(
    _app: AppHandle,
    repo: String,
    path: String,
    links: Value,
) -> Result<Value, String> {
    Ok(kb_write::kb_update_links(&repo_root()?, repo, path, links))
}

#[tauri::command]
pub fn tag_attach(
    _app: AppHandle,
    common_path: String,
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
        &common_path,
        &Value::Object(payload),
    ))
}

#[tauri::command]
pub fn tag_detach(
    _app: AppHandle,
    common_path: String,
    key: String,
) -> Result<Value, String> {
    Ok(tag_write::tag_detach(&repo_root()?, &common_path, &key))
}

#[tauri::command]
pub fn tag_update_value(
    _app: AppHandle,
    key: String,
    value: String,
) -> Result<Value, String> {
    Ok(tag_write::tag_update_value(&repo_root()?, &key, &value))
}

/// Thin Tauri/HTTP-parity wrapper around `archive_write::archive_document`.
/// Business errors stay in the Value (`error` + `_status`); do not convert to Err.
/// App note-create may pass `{ body, source_type: "note" }` (no `source_path`) → Host synthesis.
/// MCP/HTTP archive body uses `source_path` only — `document` is rejected.
pub fn archive_document_json(payload: Value) -> Result<Value, String> {
    let root = repo_root()?;
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
        if source_type == "note" {
            return match archive_write::archive_note_document(
                &root,
                body,
                &archive_write::NoteCreateOpts::default(),
            ) {
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
    Ok(archive_write::archive_document(&root, &payload))
}

#[tauri::command]
pub fn archive_document(_app: AppHandle, payload: Value) -> Result<Value, String> {
    archive_document_json(payload)
}

#[cfg(test)]
#[path = "../unit-tests/commands/sediment_kb_write.rs"]
mod sediment_kb_write_tests;

#[cfg(test)]
#[path = "../unit-tests/commands/archive_document_write.rs"]
mod archive_document_write_tests;
