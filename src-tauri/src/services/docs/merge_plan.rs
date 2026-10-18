//! Target paths for merging the documents of one project into another project of the same
//! workspace. Pure: no disk, no database.
//!
//! A document keeps its path when nothing in the target project uses it. When any of its files
//! would land on a taken path, the whole document is renamed: `-{suffix}` goes in front of the
//! `.md` of its file name, never into its folder. Translations follow the primary name, because
//! their path is derived from it (`X.md` → `X-zh.md`).

use std::collections::HashSet;

use crate::services::archive_parse::{expected_lang_common_path, is_valid_common_path};
use crate::services::spark_data::{translation_common_paths, DocRow};

const MAX_SUFFIX_TRIES: usize = 16;

/// Where one document lands in the target project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MergeTarget {
    pub doc_id: String,
    /// New `raw_path`; the translation paths follow from it.
    pub raw_path: String,
    pub digest_path: Option<String>,
    pub renamed: bool,
}

/// `-{suffix}` in front of the `.md` of `rel`; `None` when `rel` is not a `.md` path.
fn add_suffix(rel: &str, suffix: &str) -> Option<String> {
    rel.strip_suffix(".md").map(|stem| format!("{stem}-{suffix}.md"))
}

/// The raw layer files of a document whose raw path is `raw`: the primary file, then one
/// file per translation language.
fn list_raw_files(raw: &str, langs: &[String]) -> Result<Vec<String>, String> {
    let mut files = vec![raw.to_string()];
    for lang in langs {
        files.push(expected_lang_common_path(raw, lang).ok_or_else(|| format!("no translation path for `{raw}`"))?);
    }
    Ok(files)
}

fn is_free(
    raw_files: &[String],
    digest: Option<&String>,
    taken_raw: &HashSet<String>,
    taken_digest: &HashSet<String>,
) -> bool {
    !raw_files.iter().any(|p| taken_raw.contains(p)) && !digest.is_some_and(|d| taken_digest.contains(d))
}

/// One [`MergeTarget`] per document, in order. `taken_raw` and `taken_digest` are the relative
/// paths already used in the target project's raw layer (translations included) and digest
/// layer. A suffix is drawn from `next_suffix` until the document fits; paths given to one
/// document are kept from the next. Fails when a document cannot be renamed to a valid path.
pub(crate) fn plan_merge_targets(
    docs: &[DocRow],
    taken_raw: &HashSet<String>,
    taken_digest: &HashSet<String>,
    mut next_suffix: impl FnMut() -> String,
) -> Result<Vec<MergeTarget>, String> {
    let mut raw_used = taken_raw.clone();
    let mut digest_used = taken_digest.clone();
    let mut targets = Vec::with_capacity(docs.len());
    for doc in docs {
        let langs: Vec<String> = translation_common_paths(doc)?.into_iter().map(|(lang, _)| lang).collect();
        let raw_files = list_raw_files(&doc.raw_path, &langs).map_err(|e| format!("doc {}: {e}", doc.doc_id))?;
        let (raw_path, digest_path, renamed) =
            if is_free(&raw_files, doc.digest_path.as_ref(), &raw_used, &digest_used) {
                (doc.raw_path.clone(), doc.digest_path.clone(), false)
            } else {
                rename_until_free(doc, &langs, &raw_used, &digest_used, &mut next_suffix)?
            };
        raw_used.extend(list_raw_files(&raw_path, &langs)?);
        digest_used.extend(digest_path.clone());
        targets.push(MergeTarget { doc_id: doc.doc_id.clone(), raw_path, digest_path, renamed });
    }
    Ok(targets)
}

/// Draw suffixes until the renamed document fits: `(raw path, digest path, true)`.
fn rename_until_free(
    doc: &DocRow,
    langs: &[String],
    raw_used: &HashSet<String>,
    digest_used: &HashSet<String>,
    next_suffix: &mut impl FnMut() -> String,
) -> Result<(String, Option<String>, bool), String> {
    for _ in 0..MAX_SUFFIX_TRIES {
        let suffix = next_suffix();
        let raw = add_suffix(&doc.raw_path, &suffix)
            .filter(|p| is_valid_common_path(p))
            .ok_or_else(|| format!("doc {}: cannot rename `{}`", doc.doc_id, doc.raw_path))?;
        // A digest that shares the raw name keeps sharing it; any other digest name gets the suffix too.
        let digest = match doc.digest_path.as_deref() {
            Some(d) if d == doc.raw_path => Some(raw.clone()),
            Some(d) => Some(add_suffix(d, &suffix).ok_or_else(|| format!("doc {}: cannot rename `{d}`", doc.doc_id))?),
            None => None,
        };
        let raw_files = list_raw_files(&raw, langs).map_err(|e| format!("doc {}: {e}", doc.doc_id))?;
        if is_free(&raw_files, digest.as_ref(), raw_used, digest_used) {
            return Ok((raw, digest, true));
        }
    }
    Err(format!("doc {}: no free name after {MAX_SUFFIX_TRIES} tries", doc.doc_id))
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_merge_plan.rs"]
mod tests;
