use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::docs::hide_patterns::{compiled_hide_regexes, path_is_hidden};
use crate::services::docs::workspace_root;
use crate::services::docs::doc_file_path;
use crate::services::spark_data::{
    self, find_doc, get_workspace, list_doc_entries, DocRow, FoundDoc,
};

const DOC_LAYERS: &[&str] = &["raw", "digest"];

fn is_under_root(root: &Path, target: &Path) -> bool {
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let prefix = format!(
        "{}{}",
        root_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    target.to_string_lossy().starts_with(&prefix)
}

/// Sidebar index of one workspace: `{ "workspace_id": id, "entries": { doc_id: entry } }`.
/// `workspace` is an id; the default workspace when absent.
pub fn get_docs_index(workspace: Option<&str>) -> Value {
    let result = spark_data::open().and_then(|conn| {
        let row = match workspace.map(str::trim).filter(|s| !s.is_empty()) {
            Some(id) => get_workspace(&conn, id)?.ok_or_else(|| format!("Unknown workspace: {id}"))?,
            None => spark_data::list_workspaces(&conn)?
                .into_iter()
                .find(|w| w.is_default)
                .ok_or_else(|| "default workspace missing".to_string())?,
        };
        let mut entries = list_doc_entries(&conn, &row)?;
        // Hidden files: a document with a hidden path segment is not listed (still readable by id).
        let hide = compiled_hide_regexes(&row.id);
        if !hide.is_empty() {
            entries.retain(|_, entry| {
                let raw_path = entry.get("raw_path").and_then(Value::as_str).unwrap_or("");
                !path_is_hidden(raw_path, &hide)
            });
        }
        Ok(json!({ "workspace_id": row.id, "entries": entries }))
    });
    result.unwrap_or_else(|e| json!({ "error": format!("notes index: {e}"), "_status": 500 }))
}

fn load_doc(id: &str) -> Result<(rusqlite::Connection, FoundDoc), Value> {
    let id = id.trim();
    if !spark_data::is_valid_doc_id(id) {
        return Err(json!({ "error": "Invalid id", "_status": 400 }));
    }
    let conn = spark_data::open().map_err(|e| json!({ "error": e, "_status": 500 }))?;
    match find_doc(&conn, id) {
        Ok(Some(found)) => Ok((conn, found)),
        Ok(None) => Err(json!({ "error": "File not found", "_status": 404 })),
        Err(e) => Err(json!({ "error": e, "_status": 500 })),
    }
}

fn doc_rel(note: &DocRow, layer: &str, lang: Option<&str>) -> Result<String, Value> {
    match (layer, lang.map(str::trim).filter(|s| !s.is_empty())) {
        ("raw", Some(lang)) => expected_lang_common_path(&note.raw_path, lang)
            .ok_or_else(|| json!({ "error": "Invalid path", "_status": 400 })),
        ("digest", _) => Ok(note.digest_path.clone().unwrap_or_else(|| note.raw_path.clone())),
        _ => Ok(note.raw_path.clone()),
    }
}

fn note_disk_path(found: &FoundDoc, layer: &str, lang: Option<&str>) -> Result<PathBuf, Value> {
    let note = &found.doc;
    let rel = doc_rel(note, layer, lang)?;
    if rel.is_empty() || rel.contains("..") {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    doc_file_path(&workspace_root(&found.workspace)?, layer, &note.project_id, &rel)
}

/// Notes markdown body by doc id (`{layer}/{project.id}/{rel}` on disk).
pub fn get_doc_file(layer: &str, id: &str, lang: Option<&str>) -> Value {
    let layer = layer.trim();
    if !DOC_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let (_conn, found) = match load_doc(id) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let target = match note_disk_path(&found, layer, lang) {
        Ok(p) => p,
        Err(v) => return v,
    };
    read_text_file(&target)
}

fn read_text_file(target: &Path) -> Value {
    if !target.is_file() {
        return json!({
            "error": "File not found",
            "_status": 404
        });
    }
    match fs::read_to_string(target) {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

/// MIME whitelist for notes raster assets (PNG/JPEG/GIF/WebP).
pub fn mime_from_extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn doc_asset_rel(base_rel: &str, href: &str) -> Result<String, Value> {
    let href = href.trim();
    if base_rel.is_empty() || href.is_empty() {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if base_rel.contains("..") || href.contains("..") {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("data:")
        || href.starts_with("blob:")
        || href.starts_with('/')
    {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    let joined = Path::new(base_rel)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(href);
    let joined_str = joined.to_string_lossy().replace('\\', "/");
    if joined_str.is_empty() {
        return Err(json!({ "error": "Invalid path", "_status": 400 }));
    }
    Ok(joined_str)
}

/// Notes binary asset next to the note file, base64 in JSON for invoke.
pub fn get_doc_asset(layer: &str, id: &str, href: &str, lang: Option<&str>) -> Value {
    let layer = layer.trim();
    if !DOC_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let (_conn, found) = match load_doc(id) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let root = match workspace_root(&found.workspace) {
        Ok(root) => root,
        Err(v) => return v,
    };
    let base_rel = match doc_rel(&found.doc, layer, lang) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let rel = match doc_asset_rel(&base_rel, href) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let target = match doc_file_path(&root, layer, &found.doc.project_id, &rel) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let notes_canon = match root.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    if !is_under_root(&notes_canon, &target) {
        return json!({ "error": "Path traversal not allowed", "_status": 400 });
    }
    let mime = match mime_from_extension(&target) {
        Some(m) => m,
        None => {
            return json!({
                "error": "Unsupported media type",
                "_status": 400
            });
        }
    };
    if !target.is_file() {
        return json!({
            "error": "File not found",
            "_status": 404
        });
    }
    match fs::read(&target) {
        Ok(bytes) => {
            let data_b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            json!({ "data_b64": data_b64, "mime_type": mime })
        }
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

pub fn get_draft(_repo_root: &Path, id: &str) -> Value {
    let id = id.trim();
    if !spark_data::is_valid_doc_id(id) {
        return json!({ "error": "Invalid id" });
    }
    let target = match crate::config::paths::draft_path(id) {
        Ok(p) => p,
        Err(_) => return json!({ "error": "Invalid path" }),
    };
    let content = fs::read_to_string(&target).unwrap_or_default();
    json!({ "content": content })
}

/// Locate the raw note file. Success is `{id, ok, path}` only — no body text.
pub fn get_note_path_by_id(id: &str) -> Value {
    let id = id.trim();
    if id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    if !spark_data::is_valid_doc_id(id) {
        return json!({ "id": id, "ok": false, "error": "Invalid id" });
    }
    let conn = match spark_data::open() {
        Ok(c) => c,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let Some(found) = (match find_doc(&conn, id) {
        Ok(n) => n,
        Err(e) => return json!({ "id": id, "ok": false, "error": e }),
    }) else {
        return json!({ "id": id, "ok": false, "error": "Entry not found" });
    };
    let note = &found.doc;
    let root = match workspace_root(&found.workspace) {
        Ok(root) => root,
        Err(v) => return v,
    };
    let target = match doc_file_path(&root, "raw", &note.project_id, &note.raw_path) {
        Ok(p) => p,
        Err(v) => {
            let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or("Invalid path");
            return json!({ "id": id, "ok": false, "error": msg });
        }
    };
    if !target.is_file() {
        return json!({ "id": id, "ok": false, "error": "File not found" });
    }
    json!({
        "id": id,
        "ok": true,
        "path": target.to_string_lossy(),
    })
}
