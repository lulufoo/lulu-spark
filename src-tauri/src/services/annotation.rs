//! Notes annotation write handlers: done, importance, links, comments, highlights.
//! They live on the `note` row; comments and highlights belong to the raw layer.

use serde_json::{json, Map, Value};

use crate::services::note_edit::{clean_common_path, edit_note, find_note};
use crate::services::spark_data::{self, annotation_object};

use super::id::random_hex12;

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

/// Comments and highlights exist on the raw layer only.
fn check_layer(layer: &str) -> Result<(), Value> {
    let layer = layer.trim();
    match layer {
        "raw" => Ok(()),
        _ => Err(json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 })),
    }
}

fn invalid_state() -> Value {
    json!({ "error": "Invalid annotation state", "_status": 500 })
}

/// The annotation object of the note at `common_path`; `{}` when there is none.
pub fn read_annotation_object(common_path: &str) -> Value {
    let Ok(cp) = clean_common_path(common_path) else {
        return json!({});
    };
    let found = spark_data::open().ok().and_then(|conn| find_note(&conn, cp).ok());
    found.map_or_else(|| json!({}), |note| annotation_object(&note))
}

pub fn set_done(_repo_root: &std::path::Path, common_path: &str, done: bool) -> Value {
    edit_note(common_path, |_, note| {
        note.done = done;
        Ok(json!({ "ok": true }))
    })
}

pub fn set_importance(_repo_root: &std::path::Path, common_path: &str, importance: Option<String>) -> Value {
    let imp = importance.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(v) = imp {
        if !matches!(v, "high" | "medium" | "low") {
            return json!({ "error": format!("Invalid importance: {v}"), "_status": 400 });
        }
    }
    edit_note(common_path, |_, note| {
        note.importance = imp.map(String::from);
        Ok(json!({ "ok": true }))
    })
}

pub fn update_links(_repo_root: &std::path::Path, common_path: &str, links: Value) -> Value {
    let Value::Array(links_arr) = links else {
        return json!({ "error": "links must be array", "_status": 400 });
    };
    for link in &links_arr {
        let url = link.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if !is_valid_http_url(url) {
            return json!({ "error": format!("Invalid url: {url}"), "_status": 400 });
        }
    }
    edit_note(common_path, |_, note| {
        note.links = Value::Array(links_arr);
        Ok(json!({ "ok": true }))
    })
}

fn text_of(value: &Value, key: &str) -> String {
    value.get(key).and_then(|v| v.as_str()).unwrap_or("").trim().to_string()
}

pub fn update_comments(
    _repo_root: &std::path::Path,
    common_path: &str,
    layer: &str,
    comment: Value,
    ts: String,
) -> Value {
    if let Err(reply) = check_layer(layer) {
        return reply;
    }
    let (cid, text, ts) = (text_of(&comment, "id"), text_of(&comment, "text"), ts.trim().to_string());
    edit_note(common_path, |_, note| {
        let Value::Array(comments) = &mut note.comments else {
            return Err(invalid_state());
        };
        if cid.is_empty() {
            if text.is_empty() {
                return Err(json!({ "error": "text required for new comment", "_status": 400 }));
            }
            let id = random_hex12();
            comments.push(json!({ "id": id, "text": text, "ts": ts }));
            return Ok(json!({ "ok": true, "id": id }));
        }
        let Some(idx) = comments.iter().position(|c| c.get("id").and_then(|v| v.as_str()) == Some(&cid)) else {
            return Err(json!({ "error": "Comment not found", "_status": 404 }));
        };
        if text.is_empty() {
            comments.remove(idx);
            return Ok(json!({ "ok": true, "id": "" }));
        }
        if let Value::Object(c) = &mut comments[idx] {
            c.insert("text".into(), json!(text));
            if !ts.is_empty() {
                c.insert("ts".into(), json!(ts));
            }
        }
        Ok(json!({ "ok": true, "id": cid }))
    })
}

pub fn reorder_comments(
    _repo_root: &std::path::Path,
    common_path: &str,
    layer: &str,
    ids: Vec<String>,
) -> Value {
    if let Err(reply) = check_layer(layer) {
        return reply;
    }
    edit_note(common_path, |_, note| {
        let comments = note.comments.as_array().cloned().unwrap_or_default();
        let by_id: Map<String, Value> = comments
            .into_iter()
            .filter_map(|c| Some((c.get("id")?.as_str()?.to_string(), c)))
            .collect();
        let reordered: Vec<Value> = ids.iter().filter_map(|id| by_id.get(id).cloned()).collect();
        if reordered.len() != ids.len() {
            return Err(json!({ "error": "Comment id not found", "_status": 404 }));
        }
        note.comments = Value::Array(reordered);
        Ok(json!({ "ok": true }))
    })
}

fn occurrence_of(highlight: &Value) -> Option<i64> {
    let occ = highlight.get("occurrence")?;
    occ.as_i64()
        .or_else(|| occ.as_u64().map(|n| n as i64))
        .or_else(|| occ.as_f64().map(|n| n as i64))
}

pub fn update_highlights(
    _repo_root: &std::path::Path,
    common_path: &str,
    layer: &str,
    highlight: Value,
    ts: String,
) -> Value {
    if let Err(reply) = check_layer(layer) {
        return reply;
    }
    let (hid, text, ts) = (text_of(&highlight, "id"), text_of(&highlight, "text"), ts.trim().to_string());
    let occurrence = occurrence_of(&highlight);
    edit_note(common_path, |_, note| {
        let Value::Array(highlights) = &mut note.highlights else {
            return Err(invalid_state());
        };
        if !hid.is_empty() {
            highlights.retain(|h| h.get("id").and_then(|v| v.as_str()) != Some(&hid));
            return Ok(json!({ "ok": true, "id": "" }));
        }
        if text.is_empty() {
            return Err(json!({ "error": "text required for new highlight", "_status": 400 }));
        }
        let id = random_hex12();
        let mut entry = Map::new();
        entry.insert("id".into(), json!(id));
        entry.insert("text".into(), json!(text));
        entry.insert("ts".into(), json!(ts));
        if let Some(n) = occurrence {
            entry.insert("occurrence".into(), json!(n));
        }
        highlights.push(Value::Object(entry));
        Ok(json!({ "ok": true, "id": id }))
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/annotation.rs"]
mod tests;
