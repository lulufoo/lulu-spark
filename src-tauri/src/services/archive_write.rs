//! Corpus archive writes for MCP (`archive_document`, `archive_digest`).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{FixedOffset, Utc};
use serde_json::{json, Map, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::repositories::atomic_json;
use crate::services::archive_parse::{
    expected_lang_common_path, is_valid_entry_id, parse_archive_document,
};
use crate::services::id::random_entry_id;
use crate::services::source_path_allow::{self, MAX_ARCHIVE_SOURCE_BYTES};
use crate::services::todo_task;
use crate::services::translation_gate;
use crate::services::workbench_read::get_corpus_index;

/// Options for Host-side note archive shell synthesis.
/// Topic is always `inbox`; `source_type` is always `note` — callers cannot override either.
#[derive(Debug, Clone, Default)]
pub struct NoteCreateOpts {
    /// Fixed `YYYYMMDDHHMM` (UTC+8) for tests; `None` uses current time.
    pub ts: Option<String>,
}

struct TranslationDoc {
    lang: String,
    common_path: String,
    rel: String,
    content: String,
}

fn now_ts_utc8() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    Utc::now()
        .with_timezone(&tz)
        .format("%Y%m%d%H%M")
        .to_string()
}

fn format_created_at_zh(ts: &str) -> Result<String, String> {
    if ts.len() != 12 || !ts.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid ts: {ts}"));
    }
    let year: u32 = ts[0..4]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let month: u32 = ts[4..6]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let day: u32 = ts[6..8]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let hour = &ts[8..10];
    let minute = &ts[10..12];
    Ok(format!("{year}年{month}月{day}日 {hour}:{minute}"))
}

fn slugify_title(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = true;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "note".to_string()
    } else {
        out
    }
}

/// Build a minimal legal archive MD shell for a note (H1 / 创建时间 / digest nav).
/// No persistence. Topic is fixed to `inbox`.
pub fn synthesize_note_archive_document(
    body: &str,
    opts: &NoteCreateOpts,
) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    let ts = opts.ts.clone().unwrap_or_else(now_ts_utc8);
    let first = body.lines().next().unwrap_or("").trim();
    let slug = if first.is_empty() {
        "note".to_string()
    } else {
        slugify_title(first)
    };
    let title = if first.is_empty() {
        format!("{ts}-{slug}")
    } else {
        first.to_string()
    };
    let created_at_zh = format_created_at_zh(&ts)?;
    let common_path = format!("inbox/notes/{ts}-{slug}.md");
    Ok(format!(
        "# {title}\n\n\
         > 创建时间：{created_at_zh}\n\
         > 来源：note\n\
         > 导航：[digest](../../../digest/{common_path})\n\n\
         ---\n\n\
         {body}"
    ))
}

/// Synthesize note archive shell then append-only write via private markdown API.
pub fn archive_note_document(
    repo_root: &Path,
    body: &str,
    opts: &NoteCreateOpts,
) -> Result<Value, String> {
    let document = synthesize_note_archive_document(body, opts)?;
    let payload = json!({ "source_type": "note" });
    let result = archive_document_from_markdown(repo_root, &document, &payload);
    if result.get("ok") == Some(&json!(true)) {
        Ok(result)
    } else {
        let status = result
            .get("_status")
            .and_then(|v| v.as_u64())
            .unwrap_or(500);
        let error = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("archive failed");
        Err(format!("{status}: {error}"))
    }
}

fn resolve_archive_source_markdown(source_path: &str) -> Result<String, Value> {
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

fn corpus_layer_path(
    corpus: &Path,
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
    let corpus_canon = match corpus.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(json!({ "error": e.to_string(), "_status": 500 })),
    };
    Ok(corpus_canon.join(layer).join(common_path))
}

fn write_markdown_atomic(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_entries_map(repo_root: &Path) -> Result<(PathBuf, Map<String, Value>), Value> {
    let corpus = workbench_knowledge_root_path(repo_root);
    let index_path = corpus.join("index.json");
    let index_value = get_corpus_index(repo_root);
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

fn save_entries(index_path: &Path, entries: &Map<String, Value>) -> Result<(), Value> {
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

fn reject_legacy_translation_fields(payload: &Value) -> Option<Value> {
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

fn parse_translations(
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

fn rollback_written(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn dual_store_rollback(
    written: &[PathBuf],
    index_path: &Path,
    snapshot_entries: &Map<String, Value>,
) {
    rollback_written(written);
    let _ = save_entries(index_path, snapshot_entries);
}

fn parse_task_ref(payload: &Value) -> Result<Option<(String, String)>, Value> {
    let master = payload
        .get("master_task_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let sub = payload
        .get("sub_task_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match (master, sub) {
        (None, None) => Ok(None),
        (Some(m), Some(s)) => Ok(Some((m.to_string(), s.to_string()))),
        _ => Err(json!({
            "error": "master_task_id and sub_task_id must appear together",
            "_status": 400
        })),
    }
}

fn finalize_task_linked_archive(
    written: &[PathBuf],
    index_path: &Path,
    index_snapshot: &Map<String, Value>,
    master_task_id: &str,
    sub_task_id: &str,
    archive_id: &str,
) -> Value {
    let complete = todo_task::complete_sub(master_task_id, sub_task_id);
    if complete.get("_status").and_then(|v| v.as_u64()) != Some(200) {
        dual_store_rollback(written, index_path, index_snapshot);
        return complete;
    }
    let linked = todo_task::link_archive(master_task_id, sub_task_id, archive_id);
    if linked.get("_status").and_then(|v| v.as_u64()) != Some(200) {
        // link-fail asymmetry (FM-5): complete_sub already persisted; corpus rolls back only.
        dual_store_rollback(written, index_path, index_snapshot);
        return linked;
    }
    json!({ "ok": true })
}

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
fn archive_document_from_markdown(repo_root: &Path, document: &str, payload: &Value) -> Value {
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

pub fn archive_digest(repo_root: &Path, payload: &Value) -> Value {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if !is_valid_entry_id(id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }

    let digest = payload
        .get("digest")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if digest.trim().is_empty() {
        return json!({ "error": "Missing digest", "_status": 400 });
    }

    let force = payload
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let (index_path, mut entries) = match load_entries_map(repo_root) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let Some(entry_val) = entries.get(id).cloned() else {
        return json!({ "error": "Entry not found", "_status": 404 });
    };
    let Some(common_path) = entry_val.get("common_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing common_path", "_status": 500 });
    };
    if common_path.is_empty() {
        return json!({ "error": "Missing common_path", "_status": 500 });
    }

    let corpus = workbench_knowledge_root_path(repo_root);
    let raw_path = match corpus_layer_path(&corpus, "raw", common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !raw_path.is_file() {
        return json!({
            "error": format!("File not found: raw/{common_path}"),
            "_status": 404
        });
    }

    let digest_path = match corpus_layer_path(&corpus, "digest", common_path) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if digest_path.is_file() && !force {
        return json!({
            "error": format!("File already exists: digest/{common_path}"),
            "_status": 409
        });
    }

    if let Err(e) = write_markdown_atomic(&digest_path, &digest) {
        return json!({ "error": e, "_status": 500 });
    }

    let mut entry_obj = entry_val.as_object().cloned().unwrap_or_default();
    let mut layers: Vec<Value> = entry_obj
        .get("layers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !layers.iter().any(|l| l.as_str() == Some("digest")) {
        layers.push(json!("digest"));
    }
    entry_obj.insert("layers".to_string(), Value::Array(layers));
    entries.insert(id.to_string(), Value::Object(entry_obj));

    if let Err(v) = save_entries(&index_path, &entries) {
        let _ = fs::remove_file(&digest_path);
        return v;
    }

    json!({
        "ok": true,
        "id": id,
        "common_path": common_path,
        "digest_path": format!("digest/{common_path}")
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/archive_write.rs"]
mod tests;
