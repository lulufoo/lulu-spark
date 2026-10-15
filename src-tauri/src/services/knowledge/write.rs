//! KB write handlers (port `server.py` KB POST handlers).

use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::kb_safe_path;

use crate::services::id::random_hex12;

use super::doc_annotation::{edit_doc, err_status};

fn is_valid_http_url(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };
    !rest.split('/').next().unwrap_or("").is_empty()
}

fn kb_root_path(repo_root: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(knowledge_root_string(repo_root))
}

pub fn kb_save(repo_root: &Path, repo: String, path: String, content: String) -> Value {
    let kb_root = kb_root_path(repo_root);
    let target = match kb_safe_path(&kb_root, &repo, &path) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    if !target.is_file() {
        return json!({
            "error": format!("file not found: {}", path.trim()),
            "_status": 404
        });
    }
    if let Err(e) = fs::write(&target, content) {
        return json!({ "error": e.to_string(), "_status": 500 });
    }
    json!({ "ok": true })
}

pub fn kb_update_comments(
    repo_root: &Path,
    repo: String,
    path: String,
    comment: Value,
    ts: String,
) -> Value {
    edit_doc(repo_root, &repo, &path, |row| {
        let Value::Array(comments) = &mut row.comments else {
            return Err(json!({ "error": "Invalid state", "_status": 500 }));
        };
        let cid = comment
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let text = comment
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if cid.is_empty() {
            let new_id = random_hex12();
            comments.push(json!({ "id": new_id, "text": text, "ts": ts }));
            return Ok(json!({ "ok": true, "id": new_id }));
        }
        if !text.is_empty() {
            for c in comments.iter_mut() {
                if c.get("id").and_then(|v| v.as_str()) == Some(&cid) {
                    if let Value::Object(ref mut m) = c {
                        m.insert("text".into(), json!(text));
                        m.insert("ts".into(), json!(ts));
                    }
                    break;
                }
            }
        } else {
            comments.retain(|c| c.get("id").and_then(|v| v.as_str()) != Some(&cid));
        }
        Ok(json!({ "ok": true }))
    })
}

pub fn kb_reorder_comments(
    repo_root: &Path,
    repo: String,
    path: String,
    ids: Vec<String>,
) -> Value {
    edit_doc(repo_root, &repo, &path, |row| {
        let comments = row.comments.as_array().cloned().unwrap_or_default();
        let id_map: Map<String, Value> = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.get("id")?.as_str()?.to_string();
                Some((id, c))
            })
            .collect();
        let reordered: Vec<Value> = ids
            .iter()
            .filter_map(|id| id_map.get(id).cloned())
            .collect();
        if reordered.len() != ids.len() {
            return Err(json!({ "error": "Comment id not found", "_status": 404 }));
        }
        row.comments = Value::Array(reordered);
        Ok(json!({ "ok": true }))
    })
}

pub fn kb_update_highlights(
    repo_root: &Path,
    repo: String,
    path: String,
    highlight: Value,
    ts: String,
) -> Value {
    edit_doc(repo_root, &repo, &path, |row| {
        let Value::Array(highlights) = &mut row.highlights else {
            return Err(json!({ "error": "Invalid state", "_status": 500 }));
        };
        let hid = highlight
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if hid.is_empty() {
            let new_id = random_hex12();
            let mut entry = Map::new();
            entry.insert("id".into(), json!(new_id));
            entry.insert(
                "text".into(),
                json!(highlight.get("text").and_then(|v| v.as_str()).unwrap_or("")),
            );
            entry.insert(
                "occurrence".into(),
                highlight.get("occurrence").cloned().unwrap_or(json!(0)),
            );
            entry.insert("ts".into(), json!(ts));
            highlights.push(Value::Object(entry));
            return Ok(json!({ "ok": true, "id": new_id }));
        }
        highlights.retain(|h| h.get("id").and_then(|v| v.as_str()) != Some(&hid));
        Ok(json!({ "ok": true }))
    })
}

pub fn kb_update_links(
    repo_root: &Path,
    repo: String,
    path: String,
    links: Value,
) -> Value {
    let links_arr = match links {
        Value::Array(a) => a,
        _ => return json!({ "error": "links must be array", "_status": 400 }),
    };
    for link in &links_arr {
        let url = link.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if !is_valid_http_url(url) {
            return json!({
                "error": format!("Invalid url: {url}"),
                "_status": 400
            });
        }
    }
    edit_doc(repo_root, &repo, &path, |row| {
        row.links = Value::Array(links_arr);
        Ok(json!({ "ok": true }))
    })
}
