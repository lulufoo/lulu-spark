//! Walk notes + registered knowledge repos into one SourceChunk list.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};

use crate::config::roots::{knowledge_root_string, notes_root_path};
use crate::services::index_build::common::{
    extract_spark_title, knowledge_doc_id, should_skip_md, spark_doc_id, SPARK_LAYERS,
};
use crate::services::knowledge::{compiled_hide_regexes, name_is_hidden};
use crate::services::spark_read::{get_topics, load_notes_index_entries};

use super::chunk::{chunk_markdown, first_heading_title};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SourceChunk {
    pub chunk_id: String,
    pub doc_id: String,
    pub category: String,
    pub layer: String,
    pub path: String,
    pub common_path: String,
    pub repo: String,
    pub title: String,
    pub body: String,
    pub content_hash: String,
    pub created_at: String,
    /// File modification time in nanoseconds since epoch; 0 when unknown.
    pub mtime: i64,
}

impl SourceChunk {
    #[allow(clippy::too_many_arguments)]
    pub fn from_text(
        category: &str,
        layer: &str,
        path: &str,
        common_path: &str,
        repo: &str,
        doc_id: &str,
        title: &str,
        body: &str,
        created_at: &str,
        idx: usize,
        mtime: i64,
    ) -> Self {
        Self {
            chunk_id: format!("{category}:{path}:{idx}"),
            doc_id: doc_id.to_string(),
            category: category.to_string(),
            layer: layer.to_string(),
            path: path.to_string(),
            common_path: common_path.to_string(),
            repo: repo.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            content_hash: content_hash(body),
            created_at: created_at.to_string(),
            mtime,
        }
    }
}

pub fn content_hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// Modification time as nanoseconds since epoch; 0 when the file is unreadable.
pub fn file_mtime_nanos(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

pub fn path_key(abs: &Path) -> String {
    abs.to_string_lossy().replace('\\', "/")
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

fn chunks_from_text(
    category: &str,
    layer: &str,
    abs: &Path,
    common_path: &str,
    repo: &str,
    doc_id: &str,
    created_at: &str,
    body: &str,
    fallback_name: &str,
) -> Vec<SourceChunk> {
    let title =
        first_heading_title(body).unwrap_or_else(|| extract_spark_title(body, fallback_name));
    let path = path_key(abs);
    let mtime = file_mtime_nanos(abs);
    chunk_markdown(body)
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            SourceChunk::from_text(
                category,
                layer,
                &path,
                common_path,
                repo,
                doc_id,
                &title,
                &text,
                created_at,
                idx,
                mtime,
            )
        })
        .collect()
}

fn chunks_from_file(
    category: &str,
    layer: &str,
    abs: &Path,
    common_path: &str,
    repo: &str,
    doc_id: &str,
    created_at: &str,
) -> Vec<SourceChunk> {
    let Ok(body) = fs::read_to_string(abs) else {
        return Vec::new();
    };
    let name = abs.file_name().and_then(|n| n.to_str()).unwrap_or("");
    chunks_from_text(
        category,
        layer,
        abs,
        common_path,
        repo,
        doc_id,
        created_at,
        &body,
        name,
    )
}

fn notes_meta(repo_root: &Path) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let Ok(entries) = load_notes_index_entries(repo_root) else {
        return map;
    };
    for (id, entry) in entries {
        let Some(common_path) = entry.get("common_path").and_then(|v| v.as_str()) else {
            continue;
        };
        if common_path.is_empty() {
            continue;
        }
        let created_at = entry
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        map.entry(common_path.to_string())
            .or_insert((id, created_at));
    }
    map
}

fn note_identity(
    meta: &HashMap<String, (String, String)>,
    layer: &str,
    common_path: &str,
) -> (String, String) {
    meta.get(common_path)
        .cloned()
        .unwrap_or_else(|| (spark_doc_id(layer, common_path), String::new()))
}

pub fn collect_notes(repo_root: &Path) -> Vec<SourceChunk> {
    let notes = notes_root_path(repo_root);
    let meta = notes_meta(repo_root);
    let hide = compiled_hide_regexes();
    let mut chunks = Vec::new();
    for layer in SPARK_LAYERS {
        let layer_dir = notes.join(layer);
        if !layer_dir.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        walk_md(&layer_dir, &layer_dir, &hide, &mut files);
        files.sort_by(|a, b| a.1.cmp(&b.1));
        for (abs, common_path) in files {
            let (doc_id, created_at) = note_identity(&meta, layer, &common_path);
            chunks.extend(chunks_from_file(
                "notes",
                layer,
                &abs,
                &common_path,
                "",
                &doc_id,
                &created_at,
            ));
        }
    }
    chunks
}

/// One note file (`notes/<layer>/<common_path>`) → `(path key, chunks)`.
/// Chunks are empty when the file is missing, not `.md`, or a translation copy;
/// the path key is still returned so callers can drop stale rows.
pub fn note_file_chunks(repo_root: &Path, layer: &str, common_path: &str) -> (String, Vec<SourceChunk>) {
    let notes = notes_root_path(repo_root);
    let abs = notes.join(layer).join(common_path);
    let key = path_key(&abs);
    let name = abs.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !SPARK_LAYERS.contains(&layer)
        || !name.ends_with(".md")
        || should_skip_md(name)
        || !abs.is_file()
    {
        return (key, Vec::new());
    }
    let meta = notes_meta(repo_root);
    let (doc_id, created_at) = note_identity(&meta, layer, common_path);
    let chunks = chunks_from_file("notes", layer, &abs, common_path, "", &doc_id, &created_at);
    (key, chunks)
}

pub fn collect_knowledge(repo_root: &Path, force_repo: Option<&str>) -> Vec<SourceChunk> {
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
            let doc_id = knowledge_doc_id(&repo_full, &rel);
            chunks.extend(chunks_from_file(
                "knowledge",
                "",
                &abs,
                &rel,
                &repo_full,
                &doc_id,
                "",
            ));
        }
    }
    chunks
}

pub fn collect_all(repo_root: &Path) -> Vec<SourceChunk> {
    let mut chunks = collect_notes(repo_root);
    chunks.extend(collect_knowledge(repo_root, None));
    chunks
}

pub fn collect_knowledge_text(
    repo: &str,
    rel: &str,
    abs: &Path,
    body: &str,
) -> Vec<SourceChunk> {
    let doc_id = knowledge_doc_id(repo, rel);
    chunks_from_text("knowledge", "", abs, rel, repo, &doc_id, "", body, rel)
}
