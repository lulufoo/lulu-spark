//! Read-only workbench APIs aligned with `server.py` GET handlers (topics, annotations, draft, config, status).

use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use chrono::{TimeZone, Utc};
use serde_json::{json, Map, Value};

use crate::services::annotation::read_annotation_object;

pub use crate::config::meili_env::{github_user_url_string, workbench_knowledge_root_path, knowledge_corpus_root_string};
use crate::config::secrets;
use crate::config::settings;

pub fn check_workbench_knowledge_root(path: &str) -> Value {
    let p = std::path::PathBuf::from(path.trim());
    if path.trim().is_empty() {
        return json!({ "ok": false, "error": "路径为空" });
    }
    if !p.is_dir() {
        return json!({
            "ok": false,
            "error": format!("目录不存在：{}", p.display())
        });
    }
    let index_path = p.join("index.json");
    if !index_path.is_file() {
        return json!({
            "ok": false,
            "error": format!("未找到 index.json：{}", index_path.display())
        });
    }
    json!({ "ok": true })
}

pub fn infer_github_user_url(workbench_root: &str) -> Value {
    let path = std::path::PathBuf::from(workbench_root);
    serde_json::json!({
        "github_user_url": crate::config::settings::infer_github_user_url_from_workbench_root(&path)
    })
}

pub fn get_config(_repo_root: &Path) -> Value {
    let _ = _repo_root;
    let s = settings::load().unwrap_or_default();
    settings::to_config_json(
        &s,
        secrets::has_github_token(),
        secrets::has_meili_key(),
    )
}

/// Derive topics in-memory from `repo-list.json` (KNOWLEDGE_CORPUS entries + inbox virtual).
pub fn get_topics(_repo_root: &Path) -> Value {
    let cache_path = match crate::config::paths::repo_list_cache_path() {
        Ok(p) => p,
        Err(_) => return json!({ "error": "topics: cannot resolve cache_dir" }),
    };
    if !cache_path.is_file() {
        return json!({ "error": "repo-list.json not found; run ⊙ 全量同步 in the app" });
    }
    let Ok(text) = fs::read_to_string(&cache_path) else {
        return json!({ "error": "topics: cannot read repo-list.json" });
    };
    let Ok(data) = serde_json::from_str::<Value>(&text) else {
        return json!({ "error": "topics: invalid repo-list.json" });
    };

    let mut topics_list: Vec<Value> = Vec::new();
    if let Some(repos) = data.get("repos").and_then(|v| v.as_array()) {
        for repo in repos {
            if repo.get("type").and_then(|v| v.as_str()) != Some("KNOWLEDGE_CORPUS") {
                continue;
            }
            let Some(full_name) = repo.get("full_name").and_then(|v| v.as_str()) else {
                continue;
            };
            let desc = repo.get("description").and_then(|v| v.as_str()).unwrap_or("");
            topics_list.push(json!({ "repo": full_name, "description": desc }));
        }
    }
    topics_list.push(json!({ "dir": "inbox", "inbox": true }));

    let mut result = json!({
        "version": 4,
        "source": "repo-list.json",
        "defaultBranch": "main",
        "topics": topics_list,
    });
    if let Ok(meta) = cache_path.metadata().and_then(|m| m.modified()) {
        let dur = meta.duration_since(UNIX_EPOCH).unwrap_or_default();
        if let chrono::LocalResult::Single(dt) =
            Utc.timestamp_opt(dur.as_secs() as i64, dur.subsec_nanos())
        {
            if let Some(obj) = result.as_object_mut() {
                obj.insert(
                    "cached_at".to_string(),
                    json!(dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
                );
            }
        }
    }
    result
}

pub fn get_annotation(repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    let corpus = workbench_knowledge_root_path(repo_root);
    read_annotation_object(&corpus, common_path)
}

pub fn get_annotations_summary(repo_root: &Path) -> Value {
    let corpus = workbench_knowledge_root_path(repo_root);
    let index_path = corpus.join("index.json");
    let Ok(text) = fs::read_to_string(&index_path) else {
        return json!({ "error": format!("No such file: {}", index_path.display()) });
    };
    let Ok(index_data) = serde_json::from_str::<Value>(&text) else {
        return json!({ "error": "invalid index.json" });
    };
    let entries = index_data
        .get("entries")
        .cloned()
        .unwrap_or(index_data.clone());
    let entry_values: Vec<Value> = match &entries {
        Value::Object(m) => m.values().cloned().collect(),
        Value::Array(a) => a.clone(),
        _ => return json!({ "error": "invalid index entries" }),
    };
    let mut result = Map::new();
    for entry in entry_values {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let common_path = obj
            .get("common_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if common_path.is_empty() {
            continue;
        }
        let ann = read_annotation_object(&corpus, &common_path);
        let Some(ann_obj) = ann.as_object() else {
            continue;
        };
        if ann_obj.is_empty() {
            continue;
        }
        let mut summary = Map::new();
        if ann_obj.get("done").and_then(|v| v.as_bool()) == Some(true) {
            summary.insert("done".into(), json!(true));
        }
        if let Some(imp) = ann_obj.get("importance").and_then(|v| v.as_str()) {
            if matches!(imp, "high" | "medium" | "low") {
                summary.insert("importance".into(), json!(imp));
            }
        }
        if let Some(links) = ann_obj.get("links") {
            if !links.is_null() {
                summary.insert("links".into(), links.clone());
            }
        }
        for layer in ["raw", "distilled", "digest", "trace", "diagnose"] {
            let Some(layer_data) = ann_obj.get(layer).and_then(|v| v.as_object()) else {
                continue;
            };
            let Some(comments) = layer_data.get("comments").and_then(|c| c.as_array()) else {
                continue;
            };
            if !comments.is_empty() {
                let cc = summary
                    .entry("comment_counts")
                    .or_insert_with(|| json!({}));
                if let Some(m) = cc.as_object_mut() {
                    m.insert(layer.to_string(), json!(comments.len()));
                }
            }
        }
        if !summary.is_empty() {
            result.insert(common_path, Value::Object(summary));
        }
    }
    Value::Object(result)
}

const CORPUS_LAYERS: &[&str] = &["raw", "distilled", "digest", "trace", "diagnose"];

/// Workbench sidebar index (`corpus/index.json`), replaces static `GET /index.json` via server.py.
pub fn get_corpus_index(_repo_root: &Path) -> Value {
    let corpus = workbench_knowledge_root_path(_repo_root);
    let index_path = corpus.join("index.json");
    if !index_path.is_file() {
        return json!({
            "error": format!("No such file: {}", index_path.display()),
            "_status": 404
        });
    }
    let Ok(text) = fs::read_to_string(&index_path) else {
        return json!({ "error": "failed to read index.json", "_status": 500 });
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => v,
        Err(e) => json!({ "error": format!("invalid index.json: {e}"), "_status": 500 }),
    }
}

/// Corpus markdown body (`{layer}/{common_path}`), replaces static file fetch via server.py.
pub fn get_corpus_file(_repo_root: &Path, layer: &str, common_path: &str) -> Value {
    let layer = layer.trim();
    if !CORPUS_LAYERS.contains(&layer) {
        return json!({ "error": format!("Invalid layer: {layer}"), "_status": 400 });
    }
    let common_path = common_path.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let corpus = workbench_knowledge_root_path(_repo_root);
    let target = corpus.join(layer).join(common_path);
    let corpus_canon = match corpus.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
    };
    let target_canon = target.canonicalize().unwrap_or(target);
    let prefix = format!(
        "{}{}",
        corpus_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return json!({ "error": "Path traversal not allowed", "_status": 400 });
    }
    if !target_canon.is_file() {
        return json!({
            "error": format!("File not found: {layer}/{common_path}"),
            "_status": 404
        });
    }
    match fs::read_to_string(&target_canon) {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

pub fn get_draft(_repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    let target = match crate::config::paths::draft_path(common_path) {
        Ok(p) => p,
        Err(_) => return json!({ "error": "Invalid path" }),
    };
    let content = fs::read_to_string(&target).unwrap_or_default();
    json!({ "content": content })
}

pub fn categories_from_git_status(stdout: &str) -> Map<String, Value> {
    let mut categories: Map<String, Value> = Map::from_iter([
        ("new".to_string(), json!([])),
        ("modified".to_string(), json!([])),
        ("deleted".to_string(), json!([])),
        ("renamed".to_string(), json!([])),
        ("conflicted".to_string(), json!([])),
    ]);
    for line in stdout.lines() {
        let line = line.trim_end();
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        let path_part = &line[3..];
        let x = xy.chars().next().unwrap_or(' ');
        let y = xy.chars().nth(1).unwrap_or(' ');
        let mut push = |key: &str, p: String| {
            categories
                .get_mut(key)
                .and_then(|v| v.as_array_mut())
                .expect("array")
                .push(json!(p));
        };
        if matches!(xy, "UU" | "AA" | "DD" | "AU" | "UA" | "DU" | "UD") {
            push("conflicted", path_part.trim().to_string());
        } else if x == 'R' || y == 'R' {
            if path_part.contains(" -> ") {
                let parts: Vec<&str> = path_part.splitn(2, " -> ").collect();
                if parts.len() == 2 {
                    push(
                        "renamed",
                        format!("{} → {}", parts[0].trim(), parts[1].trim()),
                    );
                } else {
                    push("renamed", path_part.trim().to_string());
                }
            } else {
                push("renamed", path_part.trim().to_string());
            }
        } else if x == 'D' || y == 'D' {
            push("deleted", path_part.trim().to_string());
        } else if x == 'A' || xy == "??" {
            push("new", path_part.trim().to_string());
        } else if x == 'M' || y == 'M' {
            push("modified", path_part.trim().to_string());
        } else if xy.trim().len() >= 1 {
            push("modified", path_part.trim().to_string());
        }
    }
    categories
}

pub fn get_status(repo_root: &Path) -> Value {
    let corpus = workbench_knowledge_root_path(repo_root);
    let git_root = corpus.join(".git");
    if !git_root.exists() {
        let msg = format!("corpus is not a git repository: {}", corpus.display());
        return json!({ "error": msg });
    }
    let stdout = match crate::integrations::git::status_porcelain(&corpus) {
        Ok(s) => s,
        Err(e) => return json!({ "error": e.message }),
    };
    let mut categories = categories_from_git_status(&stdout);
    let total: usize = ["new", "modified", "deleted", "renamed", "conflicted"]
        .iter()
        .filter_map(|k| categories.get(*k).and_then(|v| v.as_array()))
        .map(|a| a.len())
        .sum();
    let ahead = crate::integrations::git::ahead_count(&corpus).unwrap_or(0);
    categories.insert("total".into(), json!(total));
    categories.insert("ahead".into(), json!(ahead));
    categories.insert(
        "workbench_knowledge_root".into(),
        json!(corpus.to_string_lossy().to_string()),
    );
    Value::Object(categories)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn git_status_categories_sample() {
        let sample = " M raw/a.md\n?? b.txt\nUU c.md\nR  old -> new\n D gone.md\n";
        let m = categories_from_git_status(sample);
        assert!(m["modified"].as_array().unwrap().len() >= 1);
        assert!(m["new"].as_array().unwrap().len() >= 1);
        assert!(m["conflicted"].as_array().unwrap().len() >= 1);
        assert!(m["renamed"].as_array().unwrap().len() >= 1);
        assert!(m["deleted"].as_array().unwrap().len() >= 1);
    }

    #[test]
    fn get_config_has_frontend_contract_keys() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let v = get_config(&root);
        assert!(v.get("workbench_knowledge_root").is_some());
        assert!(v.get("knowledge_corpus_root").is_some());
        assert!(v.get("github_user_url").is_some());
        assert!(v.get("meili_url").is_some());
        assert!(v.get("cache_dir").is_some());
        assert!(v.get("has_github_token").is_some());
    }

    #[test]
    fn get_topics_missing_repo_list_returns_error() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        let cache = dir.path().join("empty-cache");
        fs::create_dir_all(&corpus).expect("mkdir");
        fs::create_dir_all(&cache).expect("cache mkdir");
        crate::config::settings::write_test_config(dir.path(), &corpus, None);
        let cfg = crate::config::settings::load().expect("load");
        let mut s = cfg;
        s.cache_dir = cache; // points to dir with no repo-list.json
        crate::config::settings::save(&s).expect("save");
        let v = get_topics(dir.path());
        assert!(v.get("error").is_some(), "expected error, got {v:?}");
        crate::config::settings::set_test_config_dir(None);
    }

    #[test]
    fn get_topics_derives_from_repo_list() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        let cache = dir.path().join("cache");
        fs::create_dir_all(&corpus).expect("mkdir");
        fs::create_dir_all(&cache).expect("cache mkdir");
        crate::config::settings::write_test_config(dir.path(), &corpus, None);
        let cfg = crate::config::settings::load().expect("load");
        let mut s = cfg;
        s.cache_dir = cache.clone();
        crate::config::settings::save(&s).expect("save");

        let repo_list = serde_json::json!({
            "repos": [
                {"full_name": "lulufoo/kb-a", "name": "kb-a", "type": "KNOWLEDGE_CORPUS", "description": "desc a"},
                {"full_name": "lulufoo/other", "name": "other", "type": "OTHER", "description": "ignored"},
            ]
        });
        fs::write(cache.join("repo-list.json"), repo_list.to_string()).expect("write");

        let v = get_topics(dir.path());
        assert!(v.get("error").is_none(), "unexpected error: {v:?}");
        assert_eq!(v["source"], "repo-list.json");
        let topics = v["topics"].as_array().expect("topics array");
        assert!(topics.iter().any(|t| t.get("repo") == Some(&serde_json::json!("lulufoo/kb-a"))));
        assert!(!topics.iter().any(|t| t.get("repo") == Some(&serde_json::json!("lulufoo/other"))));
        assert!(topics.iter().any(|t| t.get("inbox") == Some(&serde_json::json!(true))));
        crate::config::settings::set_test_config_dir(None);
    }

    #[test]
    fn get_draft_invalid_path() {
        let dir = tempfile::tempdir().expect("tmp");
        let v = get_draft(dir.path(), "..%2Fsecret");
        assert_eq!(v["error"], "Invalid path");
    }

    #[test]
    fn check_workbench_knowledge_root_requires_dir_and_index() {
        let dir = tempfile::tempdir().expect("tmp");
        let missing = check_workbench_knowledge_root("/no/such/workbench");
        assert_eq!(missing["ok"], false);

        let no_index_dir = dir.path().join("empty");
        fs::create_dir_all(&no_index_dir).expect("mkdir");
        let no_index = check_workbench_knowledge_root(no_index_dir.to_str().unwrap());
        assert_eq!(no_index["ok"], false);
        assert!(no_index["error"].as_str().unwrap().contains("index.json"));

        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        fs::write(corpus.join("index.json"), br#"{"entries":[]}"#).expect("write");
        let ok = check_workbench_knowledge_root(corpus.to_str().unwrap());
        assert_eq!(ok["ok"], true);
    }

    #[test]
    fn get_corpus_index_reads_json() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        fs::write(corpus.join("index.json"), br#"{"entries":[]}"#).expect("write");
        crate::config::settings::write_test_config(dir.path(), &corpus, None);
        let v = get_corpus_index(dir.path());
        assert_eq!(v["entries"], json!([]));
    }

    #[test]
    fn get_corpus_file_rejects_traversal() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(corpus.join("raw")).expect("mkdir");
        crate::config::settings::write_test_config(dir.path(), &corpus, None);
        let v = get_corpus_file(dir.path(), "raw", "../index.json");
        assert_eq!(v["error"], "Invalid path");
    }

    #[test]
    fn get_corpus_file_returns_content() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        let raw = corpus.join("raw").join("a.md");
        fs::create_dir_all(raw.parent().unwrap()).expect("mkdir");
        fs::write(&raw, b"hello").expect("write");
        crate::config::settings::write_test_config(dir.path(), &corpus, None);
        let v = get_corpus_file(dir.path(), "raw", "a.md");
        assert_eq!(v["content"], "hello");
    }

    #[test]
    fn get_annotation_decodes_percent_encoding() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(corpus.join("annotations").join("ai")).expect("mkdir");
        let ann_path = corpus.join("annotations/ai/note.json");
        fs::write(&ann_path, br#"{"done":true}"#).expect("write");
        crate::config::settings::write_test_config(dir.path(), &corpus, None);
        let v = get_annotation(dir.path(), "ai/note.md");
        assert_eq!(v["done"], json!(true));
    }
}
