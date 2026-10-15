//! Prove the database holds what the old files held: every entry, annotation, tag
//! and knowledge annotation is rebuilt from the database and compared with the old file.

use std::collections::BTreeSet;
use std::fmt;

use rusqlite::Connection;
use serde_json::Value;

use crate::old_data::{OldData, OldNotes};
use crate::plan::Plan;
use crate::restore::{
    expected_entry, read_knowledge, read_notes, read_tags, restore_entry, restore_note_annotation,
};
use crate::shapes::{annotation_rel_path, normalize_note_annotation, NoteAnnotation};

#[derive(Default, Debug)]
pub struct Report {
    pub notes: usize,
    pub annotation_files: usize,
    pub comments: usize,
    pub highlights: usize,
    pub importance: usize,
    pub done: usize,
    pub notes_with_links: usize,
    pub tag_keys: usize,
    pub tag_refs: i64,
    pub knowledge_files: usize,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "notes {} / annotation files {} / comments {} / highlights {} / importance {} / done {} / \
             notes with links {} / tag keys {} / tag refs {} / knowledge annotation files {}",
            self.notes, self.annotation_files, self.comments, self.highlights, self.importance, self.done,
            self.notes_with_links, self.tag_keys, self.tag_refs, self.knowledge_files
        )
    }
}

pub struct Reconciled {
    pub diffs: Vec<String>,
    pub report: Report,
}

fn len_of(value: &Value) -> usize {
    value.as_array().map_or(0, Vec::len)
}

fn compare_notes(old: &OldNotes, conn: &Connection, out: &mut Reconciled) -> Result<(), String> {
    let db = read_notes(conn)?;
    let old_ids: BTreeSet<&String> = old.entries.keys().collect();
    let db_ids: BTreeSet<&String> = db.keys().collect();
    for id in old_ids.difference(&db_ids) {
        out.diffs.push(format!("note {id}: in index.json, not in the database"));
    }
    for id in db_ids.difference(&old_ids) {
        out.diffs.push(format!("note {id}: in the database, not in index.json"));
    }
    for (id, entry) in &old.entries {
        let Some(note) = db.get(id) else { continue };
        let (want, got) = (expected_entry(entry), restore_entry(note)?);
        if want != got {
            out.diffs.push(format!("note {id}: index entry differs\n  old: {want}\n  new: {got}"));
        }
        let rel = annotation_rel_path(&note.raw_path);
        let want = match old.annotations.get(&rel) {
            Some(file) => normalize_note_annotation(&rel, &file.value)?,
            None => NoteAnnotation::empty(),
        };
        let got = restore_note_annotation(note);
        if want != got {
            out.diffs.push(format!("note {id}: annotation differs\n  old: {want:?}\n  new: {got:?}"));
        }
    }
    let r = &mut out.report;
    r.notes = db.len();
    r.annotation_files = old.annotations.len();
    for note in db.values() {
        r.comments += len_of(&note.comments);
        r.highlights += len_of(&note.highlights);
        r.importance += usize::from(note.importance.is_some());
        r.done += usize::from(note.done);
        r.notes_with_links += usize::from(len_of(&note.links) > 0);
    }
    Ok(())
}

fn compare_tags(old: &OldNotes, conn: &Connection, out: &mut Reconciled) -> Result<(), String> {
    let db = read_tags(conn)?;
    let old_keys = old.registry.get("keys").and_then(Value::as_object).ok_or("registry: no `keys`")?;
    for (key, entry) in old_keys {
        let want = (
            entry.get("value").and_then(Value::as_str).unwrap_or_default().to_string(),
            entry.get("refs").and_then(Value::as_i64),
        );
        match db.get(key) {
            None => out.diffs.push(format!("tag {key}: in the registry, not in the database")),
            Some((value, refs)) => {
                if *value != want.0 || Some(*refs) != want.1 {
                    out.diffs.push(format!(
                        "tag {key}: registry has value {:?} refs {:?}, the database has value {value:?} and {refs} references",
                        want.0, want.1
                    ));
                }
            }
        }
    }
    for key in db.keys().filter(|k| !old_keys.contains_key(*k)) {
        out.diffs.push(format!("tag {key}: in the database, not in the registry"));
    }
    out.report.tag_keys = db.len();
    out.report.tag_refs = db.values().map(|(_, refs)| refs).sum();
    Ok(())
}

fn compare_knowledge(plan: &Plan, conn: &Connection, out: &mut Reconciled) -> Result<(), String> {
    let db = read_knowledge(conn)?;
    for item in &plan.knowledge {
        match db.get(&(item.repo.clone(), item.path.clone())) {
            None => out.diffs.push(format!("knowledge {}/{}: not in the database", item.repo, item.path)),
            Some(got) if *got != item.annotation => out.diffs.push(format!(
                "knowledge {}/{}: annotation differs\n  old: {:?}\n  new: {got:?}",
                item.repo, item.path, item.annotation
            )),
            Some(_) => {}
        }
    }
    out.report.knowledge_files = plan.knowledge.len();
    Ok(())
}

pub fn reconcile(old: &OldData, plan: &Plan, conn: &Connection) -> Result<Reconciled, String> {
    let mut out = Reconciled { diffs: Vec::new(), report: Report::default() };
    if let Some(notes) = &old.notes {
        compare_notes(notes, conn, &mut out)?;
        compare_tags(notes, conn, &mut out)?;
    }
    compare_knowledge(plan, conn, &mut out)?;
    Ok(out)
}
