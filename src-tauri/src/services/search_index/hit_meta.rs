//! Hits come out of the index as `doc_id` only; the place a document lives is looked up
//! in `spark-data.sqlite` (a `docs` hit through its workspace's document table). A hit whose
//! document is gone is dropped.

use std::collections::HashMap;

use crate::services::spark_data::{self, get_project, get_requested_doc, get_workspace};

use super::collect::DOCS_CATEGORY;
use super::query::QueryHit;

/// Where one document lives: `(path, unused repo, project id, stored doc id)`.
type Location = Option<(String, String, String, String)>;

fn locate(
    conn: &rusqlite::Connection,
    category: &str,
    workspace_id: &str,
    doc_id: &str,
) -> Result<Location, String> {
    match category {
        DOCS_CATEGORY => {
            let Some(workspace) = get_workspace(conn, workspace_id)? else {
                return Ok(None);
            };
            let Some(doc) = get_requested_doc(conn, &workspace, doc_id)? else {
                return Ok(None);
            };
            let project = get_project(conn, &doc.project_id)?.map(|p| p.id).unwrap_or_default();
            Ok(Some((doc.raw_path, String::new(), project, doc.doc_id)))
        }
        _ => Ok(None),
    }
}

/// Fill `common_path` and `topic` of every hit from the tables; drop the unresolvable.
pub fn resolve_hits(hits: Vec<QueryHit>) -> Result<Vec<QueryHit>, String> {
    if hits.is_empty() {
        return Ok(hits);
    }
    let conn = spark_data::open()?;
    let mut seen: HashMap<String, Location> = HashMap::new();
    let mut out = Vec::with_capacity(hits.len());
    for mut hit in hits {
        if !seen.contains_key(&hit.doc_id) {
            let found = locate(&conn, &hit.category, &hit.workspace_id, &hit.doc_id)?;
            seen.insert(hit.doc_id.clone(), found);
        }
        if let Some(Some((path, repo, topic, stored))) = seen.get(&hit.doc_id) {
            hit.common_path = path.clone();
            hit.repo = repo.clone();
            hit.topic = topic.clone();
            hit.doc_id = stored.clone();
            out.push(hit);
        }
    }
    Ok(out)
}
