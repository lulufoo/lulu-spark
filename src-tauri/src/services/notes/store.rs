use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::source_path_allow::{self, MAX_ARCHIVE_SOURCE_BYTES};
use crate::services::translation_gate;
use crate::services::spark_data;

pub(crate) struct TranslationDoc {
    pub lang: String,
    pub common_path: String,
    pub rel: String,
    pub content: String,
}

pub(crate) fn resolve_archive_source_markdown(source_path: &str) -> Result<String, Value> {
    let canonical =
        source_path_allow::resolve_source_path(source_path, MAX_ARCHIVE_SOURCE_BYTES)?;
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

/// `{workspaces_root}/{workspace_id}`: the one folder that holds every document of a workspace.
pub fn workspace_dir(workspaces_root: &Path, workspace_id: &str) -> Result<PathBuf, Value> {
    spark_data::validate_workspace_id(workspace_id)
        .map_err(|e| json!({ "error": e, "_status": 400 }))?;
    Ok(workspaces_root.join(workspace_id))
}

/// Disk path: `{workspace_root}/{layer}/{folder}/{rel}` where `folder` is the project folder
/// and `rel` is `{folder}/{filename}` inside it. `workspace_root` is a workspace folder
/// (see [`workspace_dir`]).
pub fn doc_file_path(
    workspace_root: &Path,
    layer: &str,
    folder: &str,
    rel: &str,
) -> Result<PathBuf, Value> {
    if folder.is_empty() || folder.contains("..") || folder.contains('/') || folder.contains('\\') {
        return Err(json!({ "error": "Invalid folder", "_status": 400 }));
    }
    if rel.is_empty() || rel.contains("..") {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if layer != "raw" && layer != "digest" {
        return Err(json!({
            "error": format!("Invalid layer: {layer}"),
            "_status": 400
        }));
    }
    let root_canon = match workspace_root.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
    Ok(root_canon.join(layer).join(folder).join(rel))
}

pub(crate) fn write_markdown_atomic(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn open_data() -> Result<Connection, Value> {
    spark_data::open().map_err(|e| json!({ "error": e, "_status": 500 }))
}

pub(crate) fn reject_legacy_translation_fields(payload: &Value) -> Option<Value> {
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

pub(crate) fn parse_translations(
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

pub(crate) fn rollback_written(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/notes_store.rs"]
mod tests;
