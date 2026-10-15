//! Turn the old files into the rows to write. Anything that does not fit the new
//! model is an error here; nothing is dropped silently.

use std::collections::{BTreeMap, BTreeSet};

use lulu_spark_lib::services::spark_data::NoteRow;
use serde_json::{json, Value};

use crate::old_data::{OldData, OldFile, OldKnowledgeRepo, OldNotes};
use crate::shapes::{
    annotation_rel_path, ENTRY_KEYS, normalize_knowledge_annotation, normalize_note_annotation, translation_path,
    KnowledgeAnnotation, NoteAnnotation,
};

pub struct KnowledgePlan {
    pub repo: String,
    pub path: String,
    pub annotation: KnowledgeAnnotation,
}

#[derive(Default)]
pub struct Plan {
    pub notes: Vec<NoteRow>,
    pub tags: Vec<(String, String)>,
    pub knowledge: Vec<KnowledgePlan>,
    /// Index fields that have no column in the new model: id → the dropped fields.
    pub dropped: BTreeMap<String, Value>,
}

fn text_field(entry: &Value, id: &str, key: &str) -> Result<Option<String>, String> {
    match entry.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!("entry {id}: `{key}` must be a string, got {other}")),
    }
}

fn read_layers(entry: &Value, id: &str) -> Result<bool, String> {
    let layers: Vec<&str> = entry
        .get("layers")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("entry {id}: `layers` must be an array"))?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    match layers.as_slice() {
        ["raw"] => Ok(false),
        ["raw", "digest"] => Ok(true),
        other => Err(format!("entry {id}: unsupported layers {other:?}")),
    }
}

fn read_translations(entry: &Value, id: &str, raw_path: &str) -> Result<Value, String> {
    let Some(map) = entry.get("translations") else {
        return Ok(json!([]));
    };
    let map = map
        .as_object()
        .ok_or_else(|| format!("entry {id}: `translations` must be an object"))?;
    let mut langs = Vec::new();
    for (lang, path) in map {
        let derived = translation_path(raw_path, lang)?;
        if path.as_str() != Some(derived.as_str()) {
            return Err(format!(
                "entry {id}: translation `{lang}` is {path}, but the new model derives `{derived}` from the raw path"
            ));
        }
        langs.push(json!(lang));
    }
    langs.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    Ok(Value::Array(langs))
}

fn plan_note(id: &str, entry: &Value, plan: &mut Plan) -> Result<NoteRow, String> {
    let object = entry.as_object().ok_or_else(|| format!("entry {id}: not an object"))?;
    let raw_path = text_field(entry, id, "common_path")?
        .filter(|p| !p.is_empty())
        .ok_or_else(|| format!("entry {id}: missing `common_path`"))?;
    let mut note = NoteRow::new(id, &raw_path);
    note.digest_path = read_layers(entry, id)?.then(|| raw_path.clone());
    note.source_type = text_field(entry, id, "source_type")?;
    note.created_at = text_field(entry, id, "created_at")?;
    note.title = text_field(entry, id, "title")?;
    note.translations = read_translations(entry, id, &raw_path)?;
    let extra: serde_json::Map<String, Value> = object
        .iter()
        .filter(|(key, _)| !ENTRY_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if !extra.is_empty() {
        plan.dropped.insert(id.to_string(), Value::Object(extra));
    }
    Ok(note)
}

fn apply_annotation(note: &mut NoteRow, ann: &NoteAnnotation) {
    note.importance = ann.importance.clone();
    note.done = ann.done;
    note.links = ann.links.clone();
    note.comments = ann.raw_comments.clone();
    note.highlights = ann.raw_highlights.clone();
    note.tags = ann.tag_keys.clone();
}

fn plan_registry(registry: &Value) -> Result<Vec<(String, String)>, String> {
    let keys = registry
        .get("keys")
        .and_then(Value::as_object)
        .ok_or("tags/registry.json: `keys` must be an object")?;
    keys.iter()
        .map(|(key, entry)| {
            let value = entry
                .get("value")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("tag {key}: missing `value`"))?;
            Ok((key.clone(), value.to_string()))
        })
        .collect()
}

fn plan_notes(old: &OldNotes, plan: &mut Plan) -> Result<(), String> {
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, entry) in &old.entries {
        let mut note = plan_note(id, entry, plan)?;
        let rel = annotation_rel_path(&note.raw_path);
        by_path.entry(rel.clone()).or_default().push(id.clone());
        if let Some(file) = old.annotations.get(&rel) {
            if by_path[&rel].len() > 1 {
                return Err(format!("annotation `{rel}` is claimed by more than one entry"));
            }
            apply_annotation(&mut note, &normalize_note_annotation(&rel, &file.value)?);
            used.insert(rel);
        }
        plan.notes.push(note);
    }
    for rel in old.annotations.keys() {
        if !used.contains(rel) {
            return Err(format!("annotation `{rel}` has no entry in index.json"));
        }
    }
    plan.tags = plan_registry(&old.registry)?;
    Ok(())
}

fn find_doc_path(repo: &OldKnowledgeRepo, rel: &str, file: &OldFile) -> Result<String, String> {
    let stem = rel.strip_suffix(".json").unwrap_or(rel);
    for candidate in [format!("{stem}.md"), stem.to_string()] {
        if repo.dir.join(&candidate).is_file() {
            return Ok(candidate);
        }
    }
    Err(format!("{}: no document for this annotation in `{}`", file.file.display(), repo.repo))
}

fn plan_knowledge(repos: &[OldKnowledgeRepo], plan: &mut Plan) -> Result<(), String> {
    for repo in repos {
        for (rel, file) in &repo.annotations {
            plan.knowledge.push(KnowledgePlan {
                repo: repo.repo.clone(),
                path: find_doc_path(repo, rel, file)?,
                annotation: normalize_knowledge_annotation(rel, &file.value)?,
            });
        }
    }
    Ok(())
}

pub fn build_plan(old: &OldData) -> Result<Plan, String> {
    let mut plan = Plan::default();
    if let Some(notes) = &old.notes {
        plan_notes(notes, &mut plan)?;
    }
    plan_knowledge(&old.knowledge, &mut plan)?;
    Ok(plan)
}
