//! Corpus annotation write handlers (port `server.py` P2 annotation POSTs).

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::config::meili_env::workbench_knowledge_root_path;
use crate::repositories::annotation_paths::annotation_json_path;
use crate::repositories::atomic_json;

const ANNOTATION_LAYERS: &[&str] = &["raw", "distilled", "digest", "trace", "diagnose"];

fn random_hex12() -> String {
    let mut h = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    std::thread::current().id().hash(&mut h);
    format!("{:012x}", h.finish() & 0xFFFF_FFFF_FFFFu64)
}

fn is_valid_http_url(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    !host.is_empty()
}

fn invalid_common_path() -> Value {
    json!({ "error": "Invalid common_path", "_status": 400 })
}

fn invalid_layer(layer: &str) -> Value {
    json!({
        "error": format!("Invalid layer: {layer}"),
        "_status": 400
    })
}

fn corpus_write_target(repo_root: &Path, common_path: &str) -> Result<(PathBuf, PathBuf), Value> {
    let cp = common_path.trim();
    if cp.is_empty() || cp.contains("..") {
        return Err(invalid_common_path());
    }
    let corpus = workbench_knowledge_root_path(repo_root);
    let Some(target) = annotation_json_path(&corpus, cp) else {
        return Err(invalid_common_path());
    };
    Ok((corpus, target))
}

fn persist_annotation(target: &Path, ann: &Value) -> Option<Value> {
    atomic_json::write_json(target, ann)
        .err()
        .map(|e| json!({ "error": e, "_status": 500 }))
}

pub fn read_annotation_object(corpus: &Path, common_path: &str) -> Value {
    let Some(p) = annotation_json_path(corpus, common_path) else {
        return json!({});
    };
    if !p.is_file() {
        return json!({});
    }
    let Ok(text) = fs::read_to_string(&p) else {
        return json!({});
    };
    serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
}

pub fn set_done(repo_root: &Path, common_path: &str, done: bool) -> Value {
    let Ok((corpus, target)) = corpus_write_target(repo_root, common_path) else {
        return invalid_common_path();
    };
    let cp = common_path.trim();
    let mut ann = read_annotation_object(&corpus, cp);
    let Value::Object(ref mut map) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    if done {
        map.insert("done".into(), json!(true));
    } else {
        map.remove("done");
    }
    if let Some(err) = persist_annotation(&target, &ann) {
        return err;
    }
    json!({ "ok": true })
}

pub fn set_importance(repo_root: &Path, common_path: &str, importance: Option<String>) -> Value {
    let Ok((corpus, target)) = corpus_write_target(repo_root, common_path) else {
        return invalid_common_path();
    };
    let imp = importance.as_deref().map(str::trim);
    if let Some(v) = imp {
        if !v.is_empty() && !matches!(v, "high" | "medium" | "low") {
            return json!({
                "error": format!("Invalid importance: {v}"),
                "_status": 400
            });
        }
    }
    let cp = common_path.trim();
    let mut ann = read_annotation_object(&corpus, cp);
    let Value::Object(ref mut map) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    if let Some(v) = imp.filter(|s| !s.is_empty()) {
        map.insert("importance".into(), json!(v));
    } else {
        map.remove("importance");
    }
    if let Some(err) = persist_annotation(&target, &ann) {
        return err;
    }
    json!({ "ok": true })
}

pub fn update_links(repo_root: &Path, common_path: &str, links: Value) -> Value {
    let Ok((corpus, target)) = corpus_write_target(repo_root, common_path) else {
        return invalid_common_path();
    };
    let links_arr = match links {
        Value::Array(a) => a,
        _ => return json!({ "error": "links must be array", "_status": 400 }),
    };
    for link in &links_arr {
        let url = link
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !is_valid_http_url(url) {
            return json!({
                "error": format!("Invalid url: {url}"),
                "_status": 400
            });
        }
    }
    let cp = common_path.trim();
    let mut ann = read_annotation_object(&corpus, cp);
    let Value::Object(ref mut map) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    if links_arr.is_empty() {
        map.remove("links");
    } else {
        map.insert("links".into(), Value::Array(links_arr));
    }
    if let Some(err) = persist_annotation(&target, &ann) {
        return err;
    }
    json!({ "ok": true })
}

pub fn update_comments(
    repo_root: &Path,
    common_path: &str,
    layer: &str,
    comment: Value,
    ts: String,
) -> Value {
    let layer = layer.trim();
    if !ANNOTATION_LAYERS.contains(&layer) {
        return invalid_layer(layer);
    }
    let Ok((corpus, target)) = corpus_write_target(repo_root, common_path) else {
        return invalid_common_path();
    };
    let cp = common_path.trim();
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
        .trim()
        .to_string();
    let ts = ts.trim().to_string();

    let mut ann = read_annotation_object(&corpus, cp);
    let Value::Object(ref mut root) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    let layer_entry = root
        .entry(layer.to_string())
        .or_insert_with(|| json!({}));
    let Value::Object(ref mut layer_data) = layer_entry else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    let comments = layer_data
        .entry("comments")
        .or_insert_with(|| json!([]));
    let Value::Array(ref mut comments) = comments else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };

    let mut out_id = cid.clone();
    if !cid.is_empty() {
        let idx = comments.iter().position(|c| c.get("id").and_then(|v| v.as_str()) == Some(&cid));
        let Some(idx) = idx else {
            return json!({ "error": "Comment not found", "_status": 404 });
        };
        if !text.is_empty() {
            if let Value::Object(ref mut c) = comments[idx] {
                c.insert("text".into(), json!(text));
                if !ts.is_empty() {
                    c.insert("ts".into(), json!(ts));
                }
            }
        } else {
            comments.remove(idx);
            out_id = String::new();
        }
    } else {
        if text.is_empty() {
            return json!({
                "error": "text required for new comment",
                "_status": 400
            });
        }
        out_id = random_hex12();
        comments.push(json!({ "id": out_id, "text": text, "ts": ts }));
    }

    if comments.is_empty() {
        layer_data.remove("comments");
    }
    if let Some(err) = persist_annotation(&target, &ann) {
        return err;
    }
    json!({ "ok": true, "id": out_id })
}

pub fn reorder_comments(
    repo_root: &Path,
    common_path: &str,
    layer: &str,
    ids: Vec<String>,
) -> Value {
    let layer = layer.trim();
    if !ANNOTATION_LAYERS.contains(&layer) {
        return invalid_layer(layer);
    }
    let Ok((corpus, target)) = corpus_write_target(repo_root, common_path) else {
        return invalid_common_path();
    };
    let cp = common_path.trim();
    let mut ann = read_annotation_object(&corpus, cp);
    let Value::Object(ref mut root) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    let layer_data = root
        .entry(layer.to_string())
        .or_insert_with(|| json!({}));
    let Value::Object(ref mut layer_map) = layer_data else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    let comments = layer_map
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
    layer_map.insert("comments".into(), Value::Array(reordered));
    if let Some(err) = persist_annotation(&target, &ann) {
        return err;
    }
    json!({ "ok": true })
}

pub fn update_highlights(
    repo_root: &Path,
    common_path: &str,
    layer: &str,
    highlight: Value,
    ts: String,
) -> Value {
    let layer = layer.trim();
    if !ANNOTATION_LAYERS.contains(&layer) {
        return invalid_layer(layer);
    }
    let Ok((corpus, target)) = corpus_write_target(repo_root, common_path) else {
        return invalid_common_path();
    };
    let cp = common_path.trim();
    let hid = highlight
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let text = highlight
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let occurrence = highlight.get("occurrence").cloned();
    let ts = ts.trim().to_string();

    let mut ann = read_annotation_object(&corpus, cp);
    let Value::Object(ref mut root) = ann else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    let layer_entry = root
        .entry(layer.to_string())
        .or_insert_with(|| json!({}));
    let Value::Object(ref mut layer_data) = layer_entry else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };
    let highlights = layer_data
        .entry("highlights")
        .or_insert_with(|| json!([]));
    let Value::Array(ref mut highlights) = highlights else {
        return json!({ "error": "Invalid annotation state", "_status": 500 });
    };

    let out_id = if !hid.is_empty() {
        highlights.retain(|h| h.get("id").and_then(|v| v.as_str()) != Some(&hid));
        if highlights.is_empty() {
            layer_data.remove("highlights");
        }
        String::new()
    } else {
        if text.is_empty() {
            return json!({
                "error": "text required for new highlight",
                "_status": 400
            });
        }
        let out_id = random_hex12();
        let mut entry = Map::new();
        entry.insert("id".into(), json!(out_id));
        entry.insert("text".into(), json!(text));
        entry.insert("ts".into(), json!(ts));
        if let Some(occ) = occurrence {
            if let Some(n) = occ.as_i64() {
                entry.insert("occurrence".into(), json!(n));
            } else if let Some(n) = occ.as_u64() {
                entry.insert("occurrence".into(), json!(n as i64));
            } else if let Some(n) = occ.as_f64() {
                entry.insert("occurrence".into(), json!(n as i64));
            }
        }
        highlights.push(Value::Object(entry));
        out_id
    };

    if layer_data.get("highlights").is_none() && layer_data.get("comments").is_none() {
        root.remove(layer);
    }

    if let Some(err) = persist_annotation(&target, &ann) {
        return err;
    }
    json!({ "ok": true, "id": out_id })
}

#[cfg(test)]
#[path = "../unit-tests/services/annotation.rs"]
mod tests;
