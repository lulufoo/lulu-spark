//! Where the files of a pulled document go in a project: the paths its package names when
//! nothing else in the project uses them, else the same names with a suffix.

use std::path::Path;

use serde_json::{json, Value};

use crate::services::doc_zip::{Landing, Unpacked};
use crate::services::docs::{collect_taken_paths, plan_merge_targets};
use crate::services::spark_data::{translation_common_paths, DocRow};

/// The landing of document `doc_id`, whose package says `package`, in project `project_id` of the
/// workspace folder `root`. `others` are the rows of the other documents of that project: the
/// files they name, and every file on disk, are taken, except the files of `existing` (the
/// document's own row, when it has one) because the pull replaces those.
///
/// The renaming rules are those of moving a document into a project: `-{suffix}` goes before the
/// `.md` of the body (and of a digest that has its own name), translations follow the body.
pub(super) fn plan_landing(
    root: &Path,
    project_id: &str,
    doc_id: &str,
    package: &Unpacked,
    existing: Option<&DocRow>,
    others: &[DocRow],
    next_suffix: impl FnMut() -> String,
) -> Result<Landing, String> {
    // Every file on disk is taken (the row of a document may be missing for a file that is there);
    // the rows of the others are added below.
    let (mut taken_raw, mut taken_digest) =
        collect_taken_paths(root, project_id, &[]).map_err(|e| error_text(&e))?;
    if let Some(row) = existing {
        taken_raw.remove(&row.raw_path);
        for (_lang, rel) in translation_common_paths(row)? {
            taken_raw.remove(&rel);
        }
        // The digest is looked for at the body path when the row names none.
        taken_digest.remove(row.digest_path.as_deref().unwrap_or(&row.raw_path));
    }
    for other in others.iter().filter(|other| other.doc_id != doc_id) {
        taken_raw.insert(other.raw_path.clone());
        taken_raw.extend(translation_common_paths(other)?.into_iter().map(|(_, rel)| rel));
        taken_digest.extend(other.digest_path.clone());
    }

    let mut wanted = DocRow::new(doc_id, &package.raw_path, project_id);
    wanted.digest_path = package.digest_path.clone();
    wanted.translations = Value::Array(package.translations.iter().map(|lang| json!(lang)).collect());
    let target = plan_merge_targets(&[wanted], &taken_raw, &taken_digest, next_suffix)?
        .pop()
        .ok_or_else(|| "no landing planned".to_string())?;
    Ok(Landing { raw_path: target.raw_path, digest_path: target.digest_path })
}

fn error_text(value: &Value) -> String {
    value.get("error").and_then(Value::as_str).unwrap_or("invalid path").to_string()
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_landing.rs"]
mod tests;
