//! The comparison: this machine's documents against the cloud's list, version by version.

use std::collections::BTreeMap;

use crate::services::cloud_sync::CloudDoc;

/// One local document: its id and its version number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDoc {
    pub doc_id: String,
    pub version: i64,
}

/// What a sync run does with one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncAction {
    /// The cloud has no such document, or an older one: send the local one, stamped `version`.
    Upload { doc_id: String, version: i64 },
    /// The cloud's is newer, or the document is not here: bring it back. `version` is the cloud's,
    /// which the local document takes.
    Pull { doc_id: String, version: i64 },
    /// Both sides have the same version.
    Skip { doc_id: String },
}

impl SyncAction {
    pub fn doc_id(&self) -> &str {
        match self {
            Self::Upload { doc_id, .. } | Self::Pull { doc_id, .. } | Self::Skip { doc_id } => doc_id,
        }
    }
}

/// One action per document id found on either side, ordered by id. Only version numbers are
/// compared, never contents; there is no conflict detection.
///
/// | local | cloud | action |
/// |---|---|---|
/// | higher | lower | upload |
/// | any | none | upload |
/// | same | same | skip |
/// | lower | higher | pull |
/// | none | any | pull |
///
/// An id is expected once on each side. If one repeats, its highest version counts.
pub fn plan(local: &[LocalDoc], cloud: &[CloudDoc]) -> Vec<SyncAction> {
    let local = highest(local.iter().map(|d| (d.doc_id.as_str(), d.version)));
    let cloud = highest(cloud.iter().map(|d| (d.document_id.as_str(), d.version)));
    let mut actions: Vec<SyncAction> = local
        .iter()
        .map(|(&id, &mine)| {
            let doc_id = id.to_string();
            match cloud.get(id) {
                None => SyncAction::Upload { doc_id, version: mine },
                Some(&theirs) if mine > theirs => SyncAction::Upload { doc_id, version: mine },
                Some(&theirs) if mine == theirs => SyncAction::Skip { doc_id },
                Some(&theirs) => SyncAction::Pull { doc_id, version: theirs },
            }
        })
        .collect();
    actions.extend(
        cloud
            .iter()
            .filter(|(id, _)| !local.contains_key(*id))
            .map(|(&id, &theirs)| SyncAction::Pull { doc_id: id.to_string(), version: theirs }),
    );
    actions.sort_by(|a, b| a.doc_id().cmp(b.doc_id()));
    actions
}

fn highest<'a>(docs: impl Iterator<Item = (&'a str, i64)>) -> BTreeMap<&'a str, i64> {
    let mut out = BTreeMap::new();
    for (id, version) in docs {
        out.entry(id).and_modify(|kept: &mut i64| *kept = (*kept).max(version)).or_insert(version);
    }
    out
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_plan.rs"]
mod tests;
