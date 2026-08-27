use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::services::archive_parse::parse_archive_document;
use crate::services::id::random_entry_id;

use super::store::{
    corpus_layer_path, dual_store_rollback, finalize_task_linked_archive, load_entries_map,
    parse_task_ref, parse_translations, reject_legacy_translation_fields, resolve_archive_source_markdown,
    rollback_written, save_entries, write_markdown_atomic,
};

/// Public archive API (HTTP / MCP): body from allow-listed `source_path` only.
pub fn archive_document(repo_root: &Path, payload: &Value) -> Value {
    if payload.get("document").is_some() {
        return json!({
            "error": "document is not supported; use source_path",
            "_status": 400
        });
    }
    let Some(source_path) = payload.get("source_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing source_path", "_status": 400 });
    };
    let document = match resolve_archive_source_markdown(source_path) {
        Ok(s) => s,
        Err(v) => return v,
    };
    if document.trim().is_empty() {
        return json!({ "error": "Source file is empty", "_status": 400 });
    }
    archive_document_from_markdown(repo_root, &document, payload)
}

/// Internal write path used by note synthesis and tests that already hold markdown.
pub(super) fn archive_document_from_markdown(repo_root: &Path, document: &str, payload: &Value) -> Value {
    let source_type = payload
        .get("source_type")
        .and_then(|v| v.as_str())
        .unwrap_or("summary")
        .trim()
        .to_string();
    if source_type.is_empty() {
        return json!({ "error": "Invalid source_type", "_status": 400 });
    }

    let parsed = match parse_archive_document(document) {
        Ok(p) => p,
        Err(e) => {
            return json!({ "error": e.message(), "_status": 400 });
        }
    };

    if let Some(err) = reject_legacy_translation_fields(payload) {
        return err;
    }
    let translations = match parse_translations(payload, &parsed.common_path, document) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let task_ref = match parse_task_ref(payload) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let corpus = workbench_knowledge_root_path(repo_root);
    let raw_path = match corpus_layer_path(&corpus, "raw", &parsed.common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if raw_path.is_file() {
        return json!({
            "error": format!("File already exists: raw/{}", parsed.common_path),
            "_status": 409
        });
    }

    let mut extra_paths: Vec<PathBuf> = Vec::new();
    for t in &translations {
        let path = match corpus_layer_path(&corpus, "raw", &t.common_path) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if path.is_file() {
            return json!({
                "error": format!("File already exists: {}", t.rel),
                "_status": 409
            });
        }
        extra_paths.push(path);
    }

    let mut written: Vec<PathBuf> = Vec::new();

    if let Err(e) = write_markdown_atomic(&raw_path, &document) {
        return json!({ "error": e, "_status": 500 });
    }
    written.push(raw_path.clone());

    for (t, path) in translations.iter().zip(extra_paths.iter()) {
        if let Err(e) = write_markdown_atomic(path, &t.content) {
            rollback_written(&written);
            return json!({ "error": e, "_status": 500 });
        }
        written.push(path.clone());
    }

    let id = random_entry_id();
    let (index_path, mut entries) = match load_entries_map(repo_root) {
        Ok(v) => v,
        Err(v) => {
            rollback_written(&written);
            return v;
        }
    };
    let index_snapshot = entries.clone();

    let mut entry = Map::new();
    entry.insert("common_path".to_string(), json!(parsed.common_path));
    entry.insert("created_at".to_string(), json!(parsed.created_at));
    entry.insert("layers".to_string(), json!(["raw"]));
    entry.insert("source_type".to_string(), json!(source_type));
    if !translations.is_empty() {
        let mut map = Map::new();
        for t in &translations {
            map.insert(t.lang.clone(), json!(t.common_path));
        }
        entry.insert("translations".to_string(), Value::Object(map));
    }
    if let Some((master_task_id, sub_task_id)) = &task_ref {
        entry.insert(
            "task_ref".to_string(),
            json!({
                "master_task_id": master_task_id,
                "sub_task_id": sub_task_id,
            }),
        );
    }
    entries.insert(id.clone(), Value::Object(entry));

    if let Err(v) = save_entries(&index_path, &entries) {
        dual_store_rollback(&written, &index_path, &index_snapshot);
        return v;
    }

    if let Some((master_task_id, sub_task_id)) = task_ref {
        let plan_result = finalize_task_linked_archive(
            &written,
            &index_path,
            &index_snapshot,
            &master_task_id,
            &sub_task_id,
            &id,
        );
        if plan_result.get("ok") != Some(&json!(true)) {
            return plan_result;
        }
    }

    let extra_rel_paths: Vec<String> = translations.iter().map(|t| t.rel.clone()).collect();
    let mut response = json!({
        "ok": true,
        "id": id,
        "common_path": parsed.common_path,
        "raw_path": format!("raw/{}", parsed.common_path),
        "created_at": parsed.created_at
    });
    if !extra_rel_paths.is_empty() {
        response["extra_paths"] = json!(extra_rel_paths);
    }
    response
}
