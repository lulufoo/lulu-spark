//! Queryer B: required literals ≥3 → docs_tri candidate paths.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rusqlite::params;

use super::chunk::char_len;
use super::collect::file_mtime_nanos;
use super::query::fts_quote;
use super::store::{index_exists, indexed_mtimes, open};

/// Result of the index-side prefilter. `indexed` maps every indexed file to the
/// mtime recorded at index time so a file edited since then is never skipped.
#[derive(Debug, Clone, Default)]
pub struct GrepPrefilter {
    pub candidates: HashSet<PathBuf>,
    pub indexed: HashMap<PathBuf, i64>,
}

/// `None` = fall back to full scan (alternation or no required literal).
pub fn extract_literals(pattern: &str) -> Option<Vec<String>> {
    let mut literals = Vec::new();
    let mut buf = String::new();
    let mut chars = pattern.chars().peekable();
    let flush = |buf: &mut String, literals: &mut Vec<String>| {
        if char_len(buf) >= 3 {
            literals.push(std::mem::take(buf));
        } else {
            buf.clear();
        }
    };
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                flush(&mut buf, &mut literals);
                let _ = chars.next();
            }
            '[' => {
                flush(&mut buf, &mut literals);
                let mut prev_bs = false;
                for n in chars.by_ref() {
                    if n == ']' && !prev_bs {
                        break;
                    }
                    prev_bs = n == '\\' && !prev_bs;
                }
            }
            '|' => {
                return None;
            }
            '?' | '*' => {
                buf.clear();
            }
            '{' => {
                let rest: String = chars.clone().collect();
                if rest.starts_with('0') || rest.starts_with(',') {
                    buf.clear();
                } else {
                    flush(&mut buf, &mut literals);
                }
                for n in chars.by_ref() {
                    if n == '}' {
                        break;
                    }
                }
            }
            '.' | '+' | '(' | ')' | '^' | '$' => {
                flush(&mut buf, &mut literals);
            }
            _ => buf.push(c),
        }
    }
    flush(&mut buf, &mut literals);
    Some(literals)
}

pub fn candidate_paths(conn: &rusqlite::Connection, literals: &[String]) -> Result<HashSet<PathBuf>, String> {
    if literals.is_empty() {
        return Ok(HashSet::new());
    }
    let expr = literals
        .iter()
        .map(|t| fts_quote(t))
        .collect::<Vec<_>>()
        .join(" OR ");
    let mut stmt = conn
        .prepare("SELECT DISTINCT path FROM docs_tri WHERE docs_tri MATCH ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![expr], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut set = HashSet::new();
    for path in rows {
        set.insert(PathBuf::from(path.map_err(|e| e.to_string())?));
    }
    Ok(set)
}

pub fn indexed_paths(conn: &rusqlite::Connection) -> Result<HashMap<PathBuf, i64>, String> {
    Ok(indexed_mtimes(conn)?
        .into_iter()
        .map(|(p, m)| (PathBuf::from(p), m))
        .collect())
}

/// `None` means walk every fence file (full scan): alternation, no literal ≥3,
/// or no index file yet. Never creates the index file as a side effect.
pub fn prefilter_or_none(cache_dir: &Path, pattern: &str) -> Option<GrepPrefilter> {
    let literals = extract_literals(pattern)?;
    if literals.is_empty() || !index_exists(cache_dir) {
        return None;
    }
    let conn = open(cache_dir).ok()?;
    let candidates = candidate_paths(&conn, &literals).ok()?;
    let indexed = indexed_paths(&conn).ok()?;
    Some(GrepPrefilter { candidates, indexed })
}

/// Skip only when the file is indexed, unchanged since indexing, and not a candidate.
pub fn should_scan_file(path: &Path, prefilter: Option<&GrepPrefilter>) -> bool {
    let Some(pre) = prefilter else {
        return true;
    };
    let key = PathBuf::from(path.to_string_lossy().replace('\\', "/"));
    let Some(indexed_mtime) = pre.indexed.get(&key).or_else(|| pre.indexed.get(path)) else {
        return true;
    };
    if file_mtime_nanos(path) > *indexed_mtime {
        return true;
    }
    pre.candidates.contains(&key) || pre.candidates.contains(path)
}
