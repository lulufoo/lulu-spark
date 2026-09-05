use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::config::roots::notes_root_path;
use crate::repositories::atomic_json;
use crate::services::archive_parse::expected_lang_common_path;
use crate::services::source_path_allow::{self, MAX_ARCHIVE_SOURCE_BYTES};
use crate::services::translation_gate;
use crate::services::workbench_read::get_notes_index;

pub(super) struct TranslationDoc {
    pub lang: String,
    pub common_path: String,
    pub rel: String,
    pub content: String,
}

pub(super) fn resolve_archive_source_markdown(source_path: &str) -> Result<String, Value> {
    let canonical =
        source_path_allow::resolve_allowed_source_path(source_path, MAX_ARCHIVE_SOURCE_BYTES)?;
    let is_md = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false);
    if !is_md {
        return Err(json!({ "error": "source_path must be a .md file", "_status": 400 }));
    }
    match fs::read_to_string(&canonical) {
        Ok(s) => Ok(s),
        Err(e) => Err(json!({ "error": e.to_string(), "_status": 500 })),
    }
}

pub(super) fn notes_layer_path(
    notes: &Path,
    layer: &str,
    common_path: &str,
) -> Result<PathBuf, Value> {
    if common_path.is_empty() || common_path.contains("..") {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if layer != "raw" && layer != "digest" {
        return Err(json!({
            "error": format!("Invalid layer: {layer}"),
            "_status": 400
        }));
    }
    let notes_canon = match notes.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
    Ok(notes_canon.join(layer).join(common_path))
}

pub(super) fn write_markdown_atomic(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn load_entries_map(repo_root: &Path) -> Result<(PathBuf, Map<String, Value>), Value> {
    let notes = notes_root_path(repo_root);
    let index_path = notes.join("index.json");
    let index_value = get_notes_index(repo_root);
    if index_value.get("_status").is_some() {
        return Err(index_value);
    }
    let entries = index_value
        .get("entries")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| index_value.as_object().cloned())
        .ok_or_else(|| json!({ "error": "invalid index entries", "_status": 500 }))?;
    Ok((index_path, entries))
}

pub(super) fn save_entries(index_path: &Path, entries: &Map<String, Value>) -> Result<(), Value> {
    let data = json!({ "entries": entries });
    if entries.is_empty() {
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
        }
        let tmp = index_path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&data)
            .map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
        fs::write(&tmp, text).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
        fs::rename(&tmp, index_path).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
        return Ok(());
    }
    atomic_json::write_json(index_path, &data).map_err(|e| json!({ "error": e, "_status": 500 }))
}

pub(super) fn reject_legacy_translation_fields(payload: &Value) -> Option<Value> {
    if payload.get("extra_documents").is_some() || payload.get("index_extra").is_some() {
        return Some(json!({
            "error": "extra_documents/index_extra removed; use translations: [{lang, content}]",
            "_status": 400
        }));
    }
    None
}

fn is_valid_lang(lang: &str) -> bool {
    lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase())
}

pub(super) fn parse_translations(
    payload: &Value,
    primary_common_path: &str,
    primary_document: &str,
) -> Result<Vec<TranslationDoc>, Value> {
    let Some(arr) = payload.get("translations").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in arr {
        let lang = item
            .get("lang")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !is_valid_lang(&lang) {
            return Err(json!({ "error": format!("invalid translations.lang: {lang}"), "_status": 400 }));
        }
        let inline = item
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let path = item
            .get("source_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let content = match (!inline.is_empty(), !path.is_empty()) {
            (true, true) => {
                return Err(json!({
                    "error": "translations: use source_path or content, not both",
                    "_status": 400
                }));
            }
            (true, false) => inline.to_string(),
            (false, true) => resolve_archive_source_markdown(path)?,
            (false, false) => {
                return Err(json!({
                    "error": "translations.content or translations.source_path required",
                    "_status": 400
                }));
            }
        };
        if content.trim().is_empty() {
            return Err(json!({ "error": "translations.content must be non-empty", "_status": 400 }));
        }
        if lang == "zh" {
            if let Err(msg) = translation_gate::check_zh_parity(primary_document, &content) {
                return Err(json!({ "error": msg, "_status": 400 }));
            }
        }
        if !seen.insert(lang.clone()) {
            return Err(json!({ "error": format!("duplicate translations.lang: {lang}"), "_status": 400 }));
        }
        let common_path = expected_lang_common_path(primary_common_path, &lang).ok_or_else(|| {
            json!({ "error": "invalid primary common_path for translation", "_status": 400 })
        })?;
        let rel = format!("raw/{common_path}");
        out.push(TranslationDoc {
            lang,
            common_path,
            rel,
            content,
        });
    }
    Ok(out)
}

pub(super) fn rollback_written(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

pub(super) fn dual_store_rollback(
    written: &[PathBuf],
    index_path: &Path,
    snapshot_entries: &Map<String, Value>,
) {
    rollback_written(written);
    let _ = save_entries(index_path, snapshot_entries);
}

