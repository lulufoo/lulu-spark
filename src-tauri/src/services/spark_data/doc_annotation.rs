//! A document row's annotations in the shape the annotation files had, which is what the
//! viewer and `/api/annotation` still read. Only non-empty fields appear.

use serde_json::{json, Map, Value};

use super::doc_row::DocRow;

fn is_empty_array(value: &Value) -> bool {
    value.as_array().map_or(true, Vec::is_empty)
}

/// `{ tag_keys?, importance?, done?, links?, raw?: { comments?, highlights? } }`.
/// Comments and highlights belong to the raw layer only.
pub fn annotation_object(note: &DocRow) -> Value {
    let mut out = Map::new();
    if !is_empty_array(&note.tags) {
        out.insert("tag_keys".into(), note.tags.clone());
    }
    if let Some(importance) = &note.importance {
        out.insert("importance".into(), json!(importance));
    }
    if note.done {
        out.insert("done".into(), json!(true));
    }
    if !is_empty_array(&note.links) {
        out.insert("links".into(), note.links.clone());
    }
    let mut raw = Map::new();
    if !is_empty_array(&note.comments) {
        raw.insert("comments".into(), note.comments.clone());
    }
    if !is_empty_array(&note.highlights) {
        raw.insert("highlights".into(), note.highlights.clone());
    }
    if !raw.is_empty() {
        out.insert("raw".into(), Value::Object(raw));
    }
    Value::Object(out)
}
