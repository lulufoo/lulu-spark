//! `manifest.json` of a package, and the checks on every path that comes out of one.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::docs::attached::validate_basename;
use crate::services::docs::store::is_valid_lang;
use crate::services::spark_data::is_valid_doc_id;

pub(super) const MANIFEST_NAME: &str = "manifest.json";
pub(super) const RAW_DIR: &str = "raw";
pub(super) const DIGEST_DIR: &str = "digest";
pub(super) const ATTACHED_DIR: &str = "attached";
const FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub format: u32,
    pub doc_id: String,
    pub raw_path: String,
    pub digest_path: Option<String>,
    /// Language codes in the order of `document.translations`.
    pub translations: Vec<String>,
}

impl Manifest {
    pub(super) fn new(
        doc_id: &str,
        raw_path: &str,
        digest_path: Option<&str>,
        translations: Vec<String>,
    ) -> Self {
        Self {
            format: FORMAT,
            doc_id: doc_id.to_string(),
            raw_path: raw_path.to_string(),
            digest_path: digest_path.map(str::to_string),
            translations,
        }
    }

    /// Read a manifest and check it against the object name the package came with.
    pub(super) fn parse(bytes: &[u8], doc_id: &str) -> Result<Self, String> {
        let manifest: Manifest =
            serde_json::from_slice(bytes).map_err(|e| format!("{MANIFEST_NAME}: {e}"))?;
        manifest.check(doc_id)?;
        Ok(manifest)
    }

    pub(super) fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(self).map_err(|e| format!("{MANIFEST_NAME}: {e}"))
    }

    pub(super) fn check(&self, doc_id: &str) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!("{MANIFEST_NAME}: unsupported format {}", self.format));
        }
        if !is_valid_doc_id(&self.doc_id) || self.doc_id != doc_id {
            return Err(format!(
                "{MANIFEST_NAME}: doc_id {:?} does not match the object name {doc_id:?}",
                self.doc_id
            ));
        }
        check_member_path(&self.raw_path)?;
        if let Some(digest) = &self.digest_path {
            check_member_path(digest)?;
        }
        let mut seen = HashSet::new();
        for lang in &self.translations {
            if !is_valid_lang(lang) {
                return Err(format!("{MANIFEST_NAME}: invalid translation language {lang:?}"));
            }
            if !seen.insert(lang) {
                return Err(format!("{MANIFEST_NAME}: duplicate translation language {lang:?}"));
            }
            if expected_lang_common_path(&self.raw_path, lang).is_none() {
                return Err(format!(
                    "{MANIFEST_NAME}: raw_path {:?} must end in .md to have translations",
                    self.raw_path
                ));
            }
        }
        Ok(())
    }

    /// The same document with its body and digest at other paths: the names translations take
    /// follow the body. The package must have a digest exactly when the new paths name one.
    pub(super) fn landed(&self, raw_path: &str, digest_path: Option<&str>) -> Result<Self, String> {
        if self.digest_path.is_some() != digest_path.is_some() {
            return Err("the new paths and the package disagree about whether there is a digest".to_string());
        }
        let moved = Self::new(&self.doc_id, raw_path, digest_path, self.translations.clone());
        moved.check(&self.doc_id)?;
        Ok(moved)
    }

    /// The paths under `raw/`: the body first, then one per translation.
    pub(super) fn raw_rels(&self) -> Vec<String> {
        let mut rels = vec![self.raw_path.clone()];
        rels.extend(
            self.translations.iter().filter_map(|lang| expected_lang_common_path(&self.raw_path, lang)),
        );
        rels
    }
}

/// A path inside `raw/` or `digest/`: relative, `/`-separated, with no `..`, no backslash and no
/// empty or `.` segment. An absolute path or `a//b` has an empty segment.
pub(super) fn check_member_path(rel: &str) -> Result<(), String> {
    let bad = rel.contains("..")
        || rel.contains('\\')
        || rel.contains('\0')
        || rel.split('/').any(|segment| segment.is_empty() || segment == ".");
    if bad {
        return Err(format!("unsafe path in package: {rel:?}"));
    }
    Ok(())
}

/// A file name inside `attached/`: one name, no subdirectory, the same rule as `.attached/`.
pub(super) fn check_attached_name(name: &str) -> Result<(), String> {
    if name != name.trim() || name.contains('\0') || validate_basename(name).is_err() {
        return Err(format!("unsafe attachment name in package: {name:?}"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_zip_manifest.rs"]
mod tests;
