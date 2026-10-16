//! Hits come out of the index as `doc_id` only; the place a document lives is looked up
//! in `spark-data.sqlite`. A hit whose document is gone, or is a `missing` knowledge
//! document, is dropped.

use std::collections::HashMap;

use crate::services::sediment_kb;
use crate::services::spark_data::{self, get_knowledge_doc, get_note, KnowledgeStatus};

use super::query::QueryHit;

/// Where one document lives: `(path relative to its root, repo as owner/name)`.
type Location = Option<(String, String)>;

/// Directory name (last segment of `owner/name`) → `owner/name`, for registered repos.
fn full_repo_names() -> HashMap<String, String> {
    sediment_kb::list_repos_for_topics()
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.repo.split('/').next_back().unwrap_or(&r.repo).to_string(), r.repo))
        .collect()
}

fn locate(
    conn: &rusqlite::Connection,
    repos: &HashMap<String, String>,
    category: &str,
    doc_id: &str,
) -> Result<Location, String> {
    match category {
        "notes" => Ok(get_note(conn, doc_id)?.map(|note| (note.raw_path, String::new()))),
        "knowledge" => Ok(get_knowledge_doc(conn, doc_id)?
            .filter(|doc| doc.status == KnowledgeStatus::Active)
            .map(|doc| {
                let repo = repos.get(&doc.repo).cloned().unwrap_or(doc.repo);
                (doc.path, repo)
            })),
        _ => Ok(None),
    }
}

/// Fill `common_path` and `repo` of every hit from the tables; drop the unresolvable.
pub fn resolve_hits(hits: Vec<QueryHit>) -> Result<Vec<QueryHit>, String> {
    if hits.is_empty() {
        return Ok(hits);
    }
    let conn = spark_data::open()?;
    let repos = if hits.iter().any(|h| h.category == "knowledge") {
        full_repo_names()
    } else {
        HashMap::new()
    };
    let mut seen: HashMap<String, Location> = HashMap::new();
    let mut out = Vec::with_capacity(hits.len());
    for mut hit in hits {
        if !seen.contains_key(&hit.doc_id) {
            let found = locate(&conn, &repos, &hit.category, &hit.doc_id)?;
            seen.insert(hit.doc_id.clone(), found);
        }
        if let Some(Some((path, repo))) = seen.get(&hit.doc_id) {
            hit.common_path = path.clone();
            hit.repo = repo.clone();
            out.push(hit);
        }
    }
    Ok(out)
}
