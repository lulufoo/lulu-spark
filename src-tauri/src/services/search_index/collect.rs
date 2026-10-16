//! Walk notes + registered knowledge repos into one SourceChunk list.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::config::roots::{knowledge_root_string, notes_root_path};
use crate::services::index_build::common::{extract_spark_title, should_skip_md};
use crate::services::knowledge::{compiled_hide_regexes, name_is_hidden};
use crate::services::notes::notes_file_path;
use crate::services::spark_data::{
    self, active_knowledge_doc_ids, category_folder, get_note, list_notes, NoteRow,
};
use crate::services::spark_read::get_topics;

use super::chunk::{chunk_markdown, first_heading_title};
use regex::Regex;

/// `(doc_id, layer, idx)`: the unique key of a chunk in the index.
pub type ChunkKey = (String, String, usize);

#[derive(Debug, Clone)]
pub struct SourceChunk {
    pub doc_id: String,
    pub category: String,
    pub layer: String,
    pub title: String,
    pub body: String,
    pub content_hash: String,
    pub created_at: String,
    /// Position of this chunk within its file.
    pub idx: usize,
}

impl SourceChunk {
    #[allow(clippy::too_many_arguments)]
    pub fn from_text(
        category: &str,
        layer: &str,
        doc_id: &str,
        title: &str,
        body: &str,
        created_at: &str,
        idx: usize,
    ) -> Self {
        Self {
            doc_id: doc_id.to_string(),
            category: category.to_string(),
            layer: layer.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            content_hash: content_hash(body),
            created_at: created_at.to_string(),
            idx,
        }
    }

    pub fn to_key(&self) -> ChunkKey {
        (self.doc_id.clone(), self.layer.clone(), self.idx)
    }
}

pub fn content_hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn walk_md(dir: &Path, base: &Path, hide: &[Regex], out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name_is_hidden(name, hide) {
            continue;
        }
        if path.is_dir() {
            walk_md(&path, base, hide, out);
        } else if path.is_file() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.ends_with(".md") && !should_skip_md(name) {
                let Some(rel) = path
                    .strip_prefix(base)
                    .ok()
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                else {
                    continue;
                };
                out.push((path, rel));
            }
        }
    }
}

fn chunks_from_file(
    category: &str,
    layer: &str,
    abs: &Path,
    doc_id: &str,
    created_at: &str,
) -> Vec<SourceChunk> {
    let Ok(body) = fs::read_to_string(abs) else {
        return Vec::new();
    };
    let name = abs.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let title = first_heading_title(&body).unwrap_or_else(|| extract_spark_title(&body, name));
    chunk_markdown(&body)
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            SourceChunk::from_text(category, layer, doc_id, &title, &text, created_at, idx)
        })
        .collect()
}

/// Chunks of one note: its raw file, plus its digest file when the row says there is one.
/// A file that is missing or unreadable contributes no chunks.
pub fn note_chunks(notes: &Path, note: &NoteRow, folder: &str) -> Vec<SourceChunk> {
    let created_at = note.created_at.as_deref().unwrap_or("");
    let mut layers = vec![("raw", note.raw_path.as_str())];
    if let Some(digest) = note.digest_path.as_deref().filter(|p| !p.is_empty()) {
        layers.push(("digest", digest));
    }
    layers
        .into_iter()
        .filter(|(_, rel)| !rel.is_empty() && !rel.contains(".."))
        .flat_map(|(layer, rel)| {
            let abs = match notes_file_path(notes, layer, folder, rel) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            };
            chunks_from_file("notes", layer, &abs, &note.doc_id, created_at)
        })
        .collect()
}

/// Chunks of every note in the `note` table.
pub fn collect_notes(repo_root: &Path) -> Result<Vec<SourceChunk>, String> {
    let notes = notes_root_path(repo_root);
    let conn = spark_data::open()?;
    let rows = list_notes(&conn)?;
    let mut chunks = Vec::new();
    for note in rows {
        let folder = category_folder(&conn, &note.category_id)?;
        chunks.extend(note_chunks(&notes, &note, &folder));
    }
    Ok(chunks)
}

/// Chunks of the note `doc_id`; empty when the row is gone (a deleted note).
pub fn note_doc_chunks(repo_root: &Path, doc_id: &str) -> Result<Vec<SourceChunk>, String> {
    let notes = notes_root_path(repo_root);
    let conn = spark_data::open()?;
    let row = get_note(&conn, doc_id)?;
    Ok(row.map_or_else(Vec::new, |note| {
        match category_folder(&conn, &note.category_id) {
            Ok(folder) => note_chunks(&notes, &note, &folder),
            Err(_) => Vec::new(),
        }
    }))
}

/// Chunks of every `active` knowledge document found on disk. A file the registry has
/// no active record for (hidden, `missing`, not scanned yet) is not indexed; the
/// record's `doc_id` is the index `doc_id`.
pub fn collect_knowledge(
    repo_root: &Path,
    force_repo: Option<&str>,
) -> Result<Vec<SourceChunk>, String> {
    let doc_ids = active_knowledge_doc_ids(&spark_data::open()?)?;
    let kb_root = PathBuf::from(knowledge_root_string(repo_root));
    let topics = get_topics(repo_root);
    let repos: Vec<(String, String)> = topics
        .get("topics")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let repo = item.get("repo")?.as_str()?.to_string();
                    let desc = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some((repo, desc))
                })
                .collect()
        })
        .unwrap_or_default();
    let hide = compiled_hide_regexes();
    let mut chunks = Vec::new();
    for (repo_full, _desc) in repos {
        if let Some(force) = force_repo {
            if repo_full != force {
                continue;
            }
        }
        let repo_name = repo_full.split('/').next_back().unwrap_or(repo_full.as_str());
        let local_dir = kb_root.join(repo_name);
        if !local_dir.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        walk_md(&local_dir, &local_dir, &hide, &mut files);
        files.sort_by(|a, b| a.1.cmp(&b.1));
        for (abs, rel) in files {
            let Some(doc_id) = doc_ids.get(&(repo_name.to_string(), rel.clone())) else {
                continue;
            };
            chunks.extend(chunks_from_file("knowledge", "", &abs, doc_id, ""));
        }
    }
    Ok(chunks)
}

pub fn collect_all(repo_root: &Path) -> Result<Vec<SourceChunk>, String> {
    let mut chunks = collect_notes(repo_root)?;
    chunks.extend(collect_knowledge(repo_root, None)?);
    Ok(chunks)
}
