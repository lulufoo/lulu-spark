//! KB write handlers (port `server.py` KB POST handlers).

use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::meili_env::knowledge_corpus_root_string;
use crate::repositories::corpus::{kb_annotation_path, kb_safe_path};

use crate::services::id::random_hex12;

fn is_valid_http_url(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };
    !rest.split('/').next().unwrap_or("").is_empty()
}

fn err_status(msg: &str) -> u16 {
    if msg.contains("invalid") || msg.contains("traversal") {
        400
    } else if msg.contains("not cloned") || msg.contains("not found") {
        404
    } else {
        500
    }
}

fn kb_ann_read(ann_path: &Path) -> Value {
    if !ann_path.is_file() {
        return json!({});
    }
    let Ok(text) = fs::read_to_string(ann_path) else {
        return json!({});
    };
    serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
}

fn kb_ann_write(ann_path: &Path, data: &Value) -> Result<(), String> {
    if let Some(parent) = ann_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(ann_path, text).map_err(|e| e.to_string())
}

fn kb_root_path(repo_root: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(knowledge_corpus_root_string(repo_root))
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
    let kb_root = kb_root_path(repo_root);
    let ann_path = match kb_annotation_path(&kb_root, &repo, &path) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    let mut ann = kb_ann_read(&ann_path);
    if !ann.is_object() {
        ann = json!({});
    }
    let Some(root) = ann.as_object_mut() else {
        return json!({ "error": "Invalid state", "_status": 500 });
    };
    let comments = root
        .entry("comments")
        .or_insert_with(|| json!([]));
    let Value::Array(ref mut comments) = comments else {
        return json!({ "error": "Invalid state", "_status": 500 });
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
        if let Err(e) = kb_ann_write(&ann_path, &ann) {
            return json!({ "error": e, "_status": 500 });
        }
        return json!({ "ok": true, "id": new_id });
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
    if let Err(e) = kb_ann_write(&ann_path, &ann) {
        return json!({ "error": e, "_status": 500 });
    }
    json!({ "ok": true })
}

pub fn kb_reorder_comments(
    repo_root: &Path,
    repo: String,
    path: String,
    ids: Vec<String>,
) -> Value {
    let kb_root = kb_root_path(repo_root);
    let ann_path = match kb_annotation_path(&kb_root, &repo, &path) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    let mut ann = kb_ann_read(&ann_path);
    let Some(root) = ann.as_object_mut() else {
        return json!({ "ok": true });
    };
    let comments = root
        .get("comments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
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
        return json!({
            "error": "Comment id not found",
            "_status": 404
        });
    }
    root.insert("comments".into(), Value::Array(reordered));
    if let Err(e) = kb_ann_write(&ann_path, &ann) {
        return json!({ "error": e, "_status": 500 });
    }
    json!({ "ok": true })
}

pub fn kb_update_highlights(
    repo_root: &Path,
    repo: String,
    path: String,
    highlight: Value,
    ts: String,
) -> Value {
    let kb_root = kb_root_path(repo_root);
    let ann_path = match kb_annotation_path(&kb_root, &repo, &path) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    let mut ann = kb_ann_read(&ann_path);
    if !ann.is_object() {
        ann = json!({});
    }
    let Some(root) = ann.as_object_mut() else {
        return json!({ "error": "Invalid state", "_status": 500 });
    };
    let highlights = root
        .entry("highlights")
        .or_insert_with(|| json!([]));
    let Value::Array(ref mut highlights) = highlights else {
        return json!({ "error": "Invalid state", "_status": 500 });
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
        if let Err(e) = kb_ann_write(&ann_path, &ann) {
            return json!({ "error": e, "_status": 500 });
        }
        return json!({ "ok": true, "id": new_id });
    }

    highlights.retain(|h| h.get("id").and_then(|v| v.as_str()) != Some(&hid));
    if let Err(e) = kb_ann_write(&ann_path, &ann) {
        return json!({ "error": e, "_status": 500 });
    }
    json!({ "ok": true })
}

pub fn kb_update_links(
    repo_root: &Path,
    repo: String,
    path: String,
    links: Value,
) -> Value {
    let kb_root = kb_root_path(repo_root);
    let ann_path = match kb_annotation_path(&kb_root, &repo, &path) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
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
    let mut ann = kb_ann_read(&ann_path);
    if !ann.is_object() {
        ann = json!({});
    }
    let Some(root) = ann.as_object_mut() else {
        return json!({ "error": "Invalid state", "_status": 500 });
    };
    if links_arr.is_empty() {
        root.remove("links");
    } else {
        root.insert("links".into(), Value::Array(links_arr));
    }
    if let Err(e) = kb_ann_write(&ann_path, &ann) {
        return json!({ "error": e, "_status": 500 });
    }
    json!({ "ok": true })
}
