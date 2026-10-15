//! The old JSON shapes, reduced to the fields the new model keeps.

use serde_json::{json, Value};

/// The `index.json` entry fields that have a place in the new model.
pub const ENTRY_KEYS: [&str; 6] = ["common_path", "created_at", "layers", "source_type", "translations", "title"];

#[derive(Debug, Clone, PartialEq)]
pub struct NoteAnnotation {
    pub tag_keys: Value,
    pub importance: Option<String>,
    pub done: bool,
    pub links: Value,
    pub raw_comments: Value,
    pub raw_highlights: Value,
}

impl NoteAnnotation {
    pub fn empty() -> Self {
        Self {
            tag_keys: json!([]),
            importance: None,
            done: false,
            links: json!([]),
            raw_comments: json!([]),
            raw_highlights: json!([]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeAnnotation {
    pub comments: Value,
    pub links: Value,
    pub highlights: Value,
}

/// `a/b.md` → `a/b.json`; anything else gets `.json` appended (the old file rule).
pub fn annotation_rel_path(common_path: &str) -> String {
    match common_path.strip_suffix(".md") {
        Some(stem) => format!("{stem}.json"),
        None => format!("{common_path}.json"),
    }
}

/// `a/b.md` + `zh` → `a/b-zh.md`: where the old index stored a translation.
pub fn translation_path(raw_path: &str, lang: &str) -> Result<String, String> {
    let stem = raw_path
        .strip_suffix(".md")
        .ok_or_else(|| format!("`{raw_path}` does not end in .md, cannot derive a translation path"))?;
    Ok(format!("{stem}-{lang}.md"))
}

fn array_field(value: &Value, key: &str, what: &str) -> Result<Value, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(json!([])),
        Some(v) if v.is_array() => Ok(v.clone()),
        Some(other) => Err(format!("{what}: `{key}` must be an array, got {other}")),
    }
}

fn reject_unknown_keys(value: &Value, allowed: &[&str], what: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{what}: not a JSON object"))?;
    match object.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(key) => Err(format!("{what}: unknown field `{key}`, the new model has no place for it")),
        None => Ok(()),
    }
}

fn read_tag_keys(value: &Value, what: &str) -> Result<Value, String> {
    let keys = array_field(value, "tag_keys", what)?;
    match keys.as_array().is_some_and(|a| a.iter().all(Value::is_string)) {
        true => Ok(keys),
        false => Err(format!("{what}: `tag_keys` must be strings")),
    }
}

fn read_importance(value: &Value, what: &str) -> Result<Option<String>, String> {
    match value.get("importance") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if matches!(s.as_str(), "high" | "medium" | "low") => Ok(Some(s.clone())),
        Some(other) => Err(format!("{what}: bad `importance` {other}")),
    }
}

fn read_done(value: &Value, what: &str) -> Result<bool, String> {
    match value.get("done") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(format!("{what}: bad `done` {other}")),
    }
}

/// `(comments, highlights)` of one layer object.
fn read_layer(value: &Value, layer: &str, what: &str) -> Result<(Value, Value), String> {
    let Some(data) = value.get(layer).filter(|v| !v.is_null()) else {
        return Ok((json!([]), json!([])));
    };
    let what = format!("{what} ({layer})");
    reject_unknown_keys(data, &["comments", "highlights"], &what)?;
    Ok((array_field(data, "comments", &what)?, array_field(data, "highlights", &what)?))
}

pub fn normalize_note_annotation(rel: &str, value: &Value) -> Result<NoteAnnotation, String> {
    let what = format!("annotation `{rel}`");
    reject_unknown_keys(value, &["tag_keys", "importance", "done", "links", "raw", "digest"], &what)?;
    let (digest_comments, digest_highlights) = read_layer(value, "digest", &what)?;
    if digest_comments != json!([]) || digest_highlights != json!([]) {
        return Err(format!("{what}: the digest layer has comments or highlights, they belong to raw only"));
    }
    let (raw_comments, raw_highlights) = read_layer(value, "raw", &what)?;
    Ok(NoteAnnotation {
        tag_keys: read_tag_keys(value, &what)?,
        importance: read_importance(value, &what)?,
        done: read_done(value, &what)?,
        links: array_field(value, "links", &what)?,
        raw_comments,
        raw_highlights,
    })
}

pub fn normalize_knowledge_annotation(rel: &str, value: &Value) -> Result<KnowledgeAnnotation, String> {
    let what = format!("knowledge annotation `{rel}`");
    reject_unknown_keys(value, &["comments", "links", "highlights"], &what)?;
    Ok(KnowledgeAnnotation {
        comments: array_field(value, "comments", &what)?,
        links: array_field(value, "links", &what)?,
        highlights: array_field(value, "highlights", &what)?,
    })
}
