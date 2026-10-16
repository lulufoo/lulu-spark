//! Hits come out of the index as `doc_id` only; the place a document lives is looked up
//! in `spark-data.sqlite`. A hit whose document is gone, or is a `missing` knowledge
//! document, is dropped.

use std::collections::HashMap;

use crate::services::spark_data::{
    self, category_folder, directory_folder, get_knowledge_doc, get_note, KnowledgeStatus,
};

use super::query::QueryHit;

/// Where one document lives: `(path relative to its root, unused repo, topic folder)`.
type Location = Option<(String, String, String)>;

fn locate(conn: &rusqlite::Connection, category: &str, doc_id: &str) -> Result<Location, String> {
    match category {
        "notes" => {
            let Some(note) = get_note(conn, doc_id)? else {
                return Ok(None);
            };
            let folder = category_folder(conn, &note.category_id).unwrap_or_default();
            Ok(Some((note.raw_path, String::new(), folder)))
        }
        "knowledge" => {
            let Some(doc) = get_knowledge_doc(conn, doc_id)?
                .filter(|doc| doc.status == KnowledgeStatus::Active)
            else {
                return Ok(None);
            };
            let folder = directory_folder(conn, &doc.directory_id)?;
            Ok(Some((doc.path, String::new(), folder)))
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
            let found = locate(&conn, &hit.category, &hit.doc_id)?;
            seen.insert(hit.doc_id.clone(), found);
        }
        if let Some(Some((path, repo, topic))) = seen.get(&hit.doc_id) {
            hit.common_path = path.clone();
            hit.repo = repo.clone();
            hit.topic = topic.clone();
            out.push(hit);
        }
    }
    Ok(out)
}
