//! Queryer A: split → route by term length → bm25 / app-layer snippet → resolve → fold.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;
use serde_json::{json, Value};

use super::chunk::char_len;
use super::hit_meta::resolve_hits;
use super::store::{index_exists, open};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryRoute {
    Empty,
    Bi { terms: Vec<String> },
    Tri {
        match_terms: Vec<String>,
        like_terms: Vec<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub category: Option<String>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryHit {
    pub doc_id: String,
    pub category: String,
    pub layer: String,
    /// Filled from `spark-data.sqlite` by `resolve_hits`: note `raw_path` or knowledge `path`.
    pub common_path: String,
    /// Filled by `resolve_hits`. Knowledge no longer uses this as identity.
    pub repo: String,
    /// Notes: category folder. Knowledge: topic folder.
    pub topic: String,
    pub title: String,
    pub body: String,
    pub snippet: String,
    pub created_at: String,
    pub score: f64,
}

pub fn classify_query(q: &str) -> QueryRoute {
    let terms: Vec<String> = q
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    if terms.is_empty() {
        return QueryRoute::Empty;
    }
    let mut match_terms = Vec::new();
    let mut like_terms = Vec::new();
    let mut only_ones = true;
    for t in terms {
        match char_len(&t) {
            0 => {}
            1 => {}
            2 => {
                only_ones = false;
                like_terms.push(t);
            }
            _ => {
                only_ones = false;
                match_terms.push(t);
            }
        }
    }
    if only_ones || (match_terms.is_empty() && like_terms.is_empty()) {
        return QueryRoute::Empty;
    }
    if match_terms.is_empty() {
        return QueryRoute::Bi { terms: like_terms };
    }
    QueryRoute::Tri {
        match_terms,
        like_terms,
    }
}

pub fn fts_quote(term: &str) -> String {
    format!("\"{}\"", term.replace('"', "\"\""))
}

fn match_expr(terms: &[String], join: &str) -> String {
    terms
        .iter()
        .map(|t| fts_quote(t))
        .collect::<Vec<_>>()
        .join(join)
}

fn like_needle(term: &str) -> String {
    format!("%{}%", term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
}

fn window_snippet(body: &str, terms: &[String]) -> String {
    let lower = body.to_lowercase();
    let mut pos = None;
    for t in terms {
        if let Some(i) = body.find(t).or_else(|| lower.find(&t.to_lowercase())) {
            pos = Some(i);
            break;
        }
    }
    let start = pos.unwrap_or(0);
    let chars: Vec<char> = body.chars().collect();
    let start_c = body[..start].chars().count().saturating_sub(24);
    let end_c = (start_c + 80).min(chars.len());
    let slice: String = chars[start_c..end_c].iter().collect();
    if start_c > 0 {
        format!("…{slice}")
    } else {
        slice
    }
}

fn apply_filters(sql: &mut String, args: &mut Vec<String>, filter: &SearchFilter) {
    if let Some(c) = &filter.category {
        sql.push_str(" AND chunk.category = ?");
        args.push(c.clone());
    }
    if let Some(l) = &filter.layer {
        sql.push_str(" AND chunk.layer = ?");
        args.push(l.clone());
    }
}

fn query_tri(
    conn: &Connection,
    match_terms: &[String],
    like_terms: &[String],
    filter: &SearchFilter,
    fetch: u32,
) -> Result<Vec<QueryHit>, String> {
    let mut sql = String::from(
        "SELECT chunk.doc_id, chunk.category, chunk.layer, chunk.title, docs_tri.body,
                snippet(docs_tri, 0, '<em>', '</em>', '…', 16), bm25(docs_tri), chunk.created_at
         FROM docs_tri JOIN chunk ON chunk.id = docs_tri.rowid
         WHERE docs_tri MATCH ?",
    );
    let mut args = vec![match_expr(match_terms, " AND ")];
    apply_filters(&mut sql, &mut args, filter);
    for t in like_terms {
        sql.push_str(" AND docs_tri.body LIKE ? ESCAPE '\\'");
        args.push(like_needle(t));
    }
    sql.push_str(&format!(" ORDER BY bm25(docs_tri) LIMIT {fetch}"));
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), row_to_hit)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn query_bi(
    conn: &Connection,
    terms: &[String],
    filter: &SearchFilter,
    fetch: u32,
) -> Result<Vec<QueryHit>, String> {
    let mut sql = String::from(
        "SELECT chunk.doc_id, chunk.category, chunk.layer, chunk.title,
                COALESCE((SELECT body FROM docs_tri WHERE rowid = chunk.id), ''),
                '', bm25(docs_bi), chunk.created_at
         FROM docs_bi JOIN chunk ON chunk.id = docs_bi.rowid
         WHERE docs_bi MATCH ?",
    );
    let mut args = vec![match_expr(terms, " AND ")];
    apply_filters(&mut sql, &mut args, filter);
    sql.push_str(&format!(" ORDER BY bm25(docs_bi) LIMIT {fetch}"));
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut hits = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), row_to_hit)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for hit in &mut hits {
        hit.snippet = window_snippet(&hit.body, terms);
    }
    Ok(hits)
}

fn row_to_hit(row: &rusqlite::Row<'_>) -> rusqlite::Result<QueryHit> {
    Ok(QueryHit {
        doc_id: row.get(0)?,
        category: row.get(1)?,
        layer: row.get(2)?,
        common_path: String::new(),
        repo: String::new(),
        topic: String::new(),
        title: row.get(3)?,
        body: row.get(4)?,
        snippet: row.get::<_, String>(5).unwrap_or_default(),
        score: row.get(6)?,
        created_at: row.get::<_, String>(7).unwrap_or_default(),
    })
}

pub fn search(
    conn: &Connection,
    q: &str,
    filter: &SearchFilter,
    limit: u32,
) -> Result<Vec<QueryHit>, String> {
    let fetch = limit.saturating_mul(8).max(limit);
    match classify_query(q) {
        QueryRoute::Empty => Ok(Vec::new()),
        QueryRoute::Tri {
            match_terms,
            like_terms,
        } => query_tri(conn, &match_terms, &like_terms, filter, fetch),
        QueryRoute::Bi { terms } => query_bi(conn, &terms, filter, fetch),
    }
}

pub fn fold_by_doc_id(hits: Vec<QueryHit>, limit: u32) -> Vec<QueryHit> {
    let mut best: HashMap<String, QueryHit> = HashMap::new();
    let mut order = Vec::new();
    for hit in hits {
        match best.get(&hit.doc_id) {
            Some(prev) if prev.score <= hit.score => {}
            _ => {
                if !best.contains_key(&hit.doc_id) {
                    order.push(hit.doc_id.clone());
                }
                best.insert(hit.doc_id.clone(), hit);
            }
        }
    }
    let mut out: Vec<QueryHit> = order
        .into_iter()
        .filter_map(|id| best.remove(&id))
        .collect();
    out.truncate(limit as usize);
    out
}

pub fn search_in_cache(
    cache_dir: &Path,
    q: &str,
    filter: &SearchFilter,
    limit: u32,
) -> Result<Vec<QueryHit>, String> {
    let conn = open(cache_dir)?;
    let hits = resolve_hits(search(&conn, q, filter, limit)?)?;
    Ok(fold_by_doc_id(hits, limit))
}

pub fn parse_limit(limit: Option<u32>, default: u32) -> u32 {
    match limit {
        None | Some(0) => default,
        Some(n) => n.min(50),
    }
}

pub fn search_desktop_spark(cache_dir: &Path, q: &str, limit: Option<u32>) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "q parameter required" });
    }
    let limit = parse_limit(limit, 10);
    if !index_exists(cache_dir) {
        return json!({ "hits": [], "error": "not_indexed" });
    }
    let filter = SearchFilter {
        category: Some("notes".into()),
        ..SearchFilter::default()
    };
    match search_in_cache(cache_dir, q, &filter, limit) {
        Ok(hits) => json!({
            "hits": hits
                .into_iter()
                .filter(|h| h.category == "notes")
                .map(|h| {
                    // digest chunks only help recall; the UI always opens the note itself.
                    json!({
                        "title": h.title,
                        "id": h.doc_id,
                        "layer": "raw",
                        "topic": h.topic,
                        "body": h.snippet,
                        "_formatted": { "body": h.snippet },
                    })
                })
                .collect::<Vec<_>>()
        }),
        Err(e) => json!({ "hits": [], "error": e }),
    }
}

pub fn search_desktop_knowledge(cache_dir: &Path, q: &str, limit: Option<u32>) -> Value {
    let q = q.trim();
    if q.is_empty() {
        return json!({ "error": "q parameter required" });
    }
    let limit = parse_limit(limit, 10);
    if !index_exists(cache_dir) {
        return json!({ "hits": [], "error": "not_indexed" });
    }
    let filter = SearchFilter {
        category: Some("knowledge".into()),
        ..SearchFilter::default()
    };
    match search_in_cache(cache_dir, q, &filter, limit) {
        Ok(hits) => json!({
            "hits": hits
                .into_iter()
                .map(|h| {
                    json!({
                        "title": h.title,
                        "id": h.doc_id,
                        "topic": h.topic,
                        "body": h.snippet,
                        "_formatted": { "body": h.snippet },
                    })
                })
                .collect::<Vec<_>>()
        }),
        Err(e) => json!({ "hits": [], "error": e }),
    }
}

pub fn cache_dir_or_err() -> Result<std::path::PathBuf, String> {
    crate::config::paths::cache_dir().map_err(|e| format!("{e:?}"))
}
