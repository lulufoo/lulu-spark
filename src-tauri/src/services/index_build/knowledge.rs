//! Knowledge Meilisearch index (`build_knowledge_index.py`).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::{json, Value};

use crate::config::meili_env::knowledge_corpus_root_string;
use crate::config::paths;
use crate::integrations::git;
use crate::integrations::search::{build_knowledge_document, MeiliAdminError, MeiliBackend};
use crate::services::workbench_read::get_topics;

use super::common::{should_skip_md, upsert_batches, BATCH_SIZE};

fn walk_kb_md_files(dir: &Path, base: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_kb_md_files(&path, base, out);
        } else if path.is_file() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.ends_with(".md") && !should_skip_md(name) {
                if let Ok(rel) = path.strip_prefix(base) {
                    out.push((path.to_path_buf(), rel.to_string_lossy().replace('\\', "/")));
                }
            }
        }
    }
}

fn collect_repo_documents(
    kb_root: &Path,
    repo_full: &str,
    topic_desc: &str,
) -> Vec<Value> {
    let repo_name = repo_full.split('/').next_back().unwrap_or(repo_full);
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return Vec::new();
    }
    let mut files = Vec::new();
    walk_kb_md_files(&local_dir, &local_dir, &mut files);
    files.sort_by(|a, b| a.1.cmp(&b.1));
    let mut docs = Vec::new();
    for (path, rel_path) in files {
        let Ok(body) = fs::read_to_string(&path) else {
            continue;
        };
        docs.push(build_knowledge_document(repo_full, &rel_path, &body, topic_desc));
    }
    docs
}

fn repo_head(local_dir: &Path) -> Option<String> {
    let out = git::exec(local_dir, &["rev-parse", "HEAD"]).ok()?;
    if out.success {
        let h = out.stdout.trim().to_string();
        if !h.is_empty() {
            return Some(h);
        }
    }
    None
}

fn commit_cache_path() -> Result<PathBuf, String> {
    paths::cache_dir()
        .map(|p| p.join("repo-commits.json"))
        .map_err(|e| format!("{e:?}"))
}

fn load_commit_cache(path: &Path) -> serde_json::Map<String, Value> {
    if !path.is_file() {
        return serde_json::Map::new();
    }
    let Ok(text) = fs::read_to_string(path) else {
        return serde_json::Map::new();
    };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn save_commit_cache(path: &Path, cache: &serde_json::Map<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn meili_meta_path() -> Result<PathBuf, String> {
    paths::cache_dir()
        .map(|p| p.join("meili-meta.json"))
        .map_err(|e| format!("{e:?}"))
}

pub fn rebuild(repo_root: &Path, wipe: bool, force_repo: Option<&str>) -> Result<String, String> {
    let meili = MeiliBackend::new(repo_root);
    meili
        .require_health()
        .map_err(MeiliAdminError::into_message)?;

    let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
    let cache_path = commit_cache_path()?;

    if wipe {
        meili.wipe_index("knowledge").map_err(MeiliAdminError::into_message)?;
        let _ = fs::remove_file(&cache_path);
    } else if let Some(repo) = force_repo {
        let repo_name = repo.split('/').next_back().unwrap_or(repo);
        let mut cache = load_commit_cache(&cache_path);
        cache.remove(repo_name);
        save_commit_cache(&cache_path, &cache)?;
    }

    meili
        .ensure_index("knowledge", "id")
        .map_err(MeiliAdminError::into_message)?;
    meili
        .put_settings(
            "knowledge",
            "searchable-attributes",
            &json!(["title", "topic_desc", "body"]),
        )
        .map_err(MeiliAdminError::into_message)?;
    meili
        .put_settings(
            "knowledge",
            "filterable-attributes",
            &json!(["repo"]),
        )
        .map_err(MeiliAdminError::into_message)?;

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

    let commit_cache = load_commit_cache(&cache_path);
    let mut new_cache = serde_json::Map::new();
    let mut total = 0usize;
    let mut log_lines = Vec::new();

    for (repo_full, topic_desc) in &repos {
        if let Some(force) = force_repo {
            if repo_full != force {
                if let Some(h) = commit_cache.get(repo_full.split('/').next_back().unwrap_or(force)) {
                    new_cache.insert(
                        repo_full.split('/').next_back().unwrap_or(force).to_string(),
                        h.clone(),
                    );
                }
                continue;
            }
        }

        let repo_name = repo_full.split('/').next_back().unwrap_or(repo_full.as_str());
        let local_dir = kb_root.join(repo_name);
        if !local_dir.is_dir() {
            log_lines.push(format!("  [skip] {repo_name}: local dir not found"));
            continue;
        }

        let head = repo_head(&local_dir);
        let cached_head = commit_cache
            .get(repo_name)
            .and_then(|v| v.as_str())
            .map(String::from);

        if !wipe && force_repo.is_none() {
            if let (Some(h), Some(c)) = (&head, &cached_head) {
                if h == c {
                    log_lines.push(format!("  [skip] {repo_name}: commit unchanged ({})", &h[..8.min(h.len())]));
                    new_cache.insert(repo_name.to_string(), json!(h));
                    continue;
                }
            }
        }

        let old_short = cached_head
            .as_ref()
            .map(|h| &h[..8.min(h.len())])
            .unwrap_or("new");
        let new_short = head
            .as_ref()
            .map(|h| &h[..8.min(h.len())])
            .unwrap_or("?");
        log_lines.push(format!("  [index] {repo_name}: {old_short} → {new_short}"));

        let filter = format!("repo = '{repo_full}'");
        meili
            .delete_documents_by_filter("knowledge", &filter)
            .map_err(MeiliAdminError::into_message)?;

        let docs = collect_repo_documents(&kb_root, repo_full, topic_desc);
        let count = docs.len();
        let batches: Vec<Vec<Value>> = docs.chunks(BATCH_SIZE).map(|c| c.to_vec()).collect();
        upsert_batches(&batches, |batch| {
            meili
                .put_documents("knowledge", batch)
                .map_err(MeiliAdminError::into_message)
        })?;
        total += count;
        log_lines.push(format!("    {count} files indexed"));

        if let Some(h) = head {
            new_cache.insert(repo_name.to_string(), json!(h));
        }
    }

    save_commit_cache(&cache_path, &new_cache)?;

    if let Ok(meta_path) = meili_meta_path() {
        if let Some(parent) = meta_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let meta = json!({
            "last_indexed_at": Utc::now().to_rfc3339(),
            "doc_count": total,
        });
        if let Ok(text) = serde_json::to_string_pretty(&meta) {
            let _ = fs::write(meta_path, format!("{text}\n"));
        }
    }

    log_lines.push(format!("Total indexed this run: {total} documents"));
    Ok(log_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_repo_docs_builds_ids() {
        let dir = tempfile::tempdir().expect("tmp");
        let kb = dir.path().join("myrepo");
        let md = kb.join("docs/a.md");
        fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
        fs::write(&md, "# Title\n\nx").expect("write");
        let docs = collect_repo_documents(dir.path(), "o/myrepo", "desc");
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["title"], "Title");
        assert_eq!(docs[0]["repo"], "o/myrepo");
    }
}
