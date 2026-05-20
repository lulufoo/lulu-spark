//! Read-only workbench APIs aligned with `server.py` GET handlers (topics, annotations, draft, config, status).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use chrono::{TimeZone, Utc};
use serde_json::{json, Map, Value};

use crate::services::annotation::read_annotation_object;

pub use crate::config::meili_env::{corpus_github_string, corpus_root_path, kb_root_string};

pub fn get_config(repo_root: &Path) -> Value {
    let corpus = corpus_root_path(repo_root);
    json!({
        "archive_root": corpus.to_string_lossy(),
        "kb_root": kb_root_string(repo_root),
        "corpus_github": corpus_github_string(repo_root),
    })
}

fn topics_cache_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".cache").join("topics.json")
}

fn topics_legacy_path(repo_root: &Path) -> PathBuf {
    repo_root.join("topics.json")
}

pub fn get_topics(repo_root: &Path) -> Value {
    let primary = topics_cache_path(repo_root);
    let legacy = topics_legacy_path(repo_root);
    let (path, used_primary) = if primary.is_file() {
        (primary.clone(), true)
    } else if legacy.is_file() {
        (legacy, false)
    } else {
        return json!({
            "error": (
                "topics.json not found; run ⊙ 全量同步 or \
                 update_topics_from_github.py"
            ),
        });
    };

    let Ok(text) = fs::read_to_string(&path) else {
        return json!({ "error": "topics.json not found" });
    };
    let Ok(mut data) = serde_json::from_str::<Value>(&text) else {
        return json!({ "error": "invalid topics.json" });
    };
    if let Some(obj) = data.as_object_mut() {
        if used_primary && primary.is_file() {
            if let Ok(meta) = primary.metadata().and_then(|m| m.modified()) {
                let dur = meta.duration_since(UNIX_EPOCH).unwrap_or_default();
                if let chrono::LocalResult::Single(dt) =
                    Utc.timestamp_opt(dur.as_secs() as i64, dur.subsec_nanos())
                {
                    obj.insert(
                        "cached_at".to_string(),
                        json!(dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
                    );
                }
            }
        }
    }
    data
}

pub fn get_annotation(repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    let corpus = corpus_root_path(repo_root);
    read_annotation_object(&corpus, common_path)
}

pub fn get_annotations_summary(repo_root: &Path) -> Value {
    let corpus = corpus_root_path(repo_root);
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

pub fn get_draft(repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    let drafts_dir = repo_root.join(".cache").join("drafts");
    let mut target = drafts_dir.clone();
    for comp in Path::new(common_path).components() {
        match comp {
            std::path::Component::Normal(s) => target.push(s),
            _ => return json!({ "error": "Invalid path" }),
        }
    }
    if !target.starts_with(&drafts_dir) {
        return json!({ "error": "Path traversal not allowed" });
    }
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
    let corpus = corpus_root_path(repo_root);
    let git_root = corpus.join(".git");
    if !git_root.exists() {
        let msg = format!("corpus is not a git repository: {}", corpus.display());
        return json!({ "error": msg });
    }
    let status_out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&corpus)
        .output();
    let Ok(out) = status_out else {
        return json!({ "error": "git status failed" });
    };
    if !out.status.success() {
        return json!({ "error": "git status failed" });
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut categories = categories_from_git_status(&stdout);
    let total: usize = ["new", "modified", "deleted", "renamed", "conflicted"]
        .iter()
        .filter_map(|k| categories.get(*k).and_then(|v| v.as_array()))
        .map(|a| a.len())
        .sum();
    let ahead_r = Command::new("git")
        .args(["rev-list", "--count", "HEAD...@{u}"])
        .current_dir(&corpus)
        .output();
    let ahead = if let Ok(a) = ahead_r {
        if a.status.success() {
            String::from_utf8_lossy(&a.stdout)
                .trim()
                .parse::<i64>()
                .unwrap_or(0) as i64
        } else {
            0
        }
    } else {
        0
    };
    categories.insert("total".into(), json!(total));
    categories.insert("ahead".into(), json!(ahead));
    categories.insert(
        "archive_root".into(),
        json!(corpus.to_string_lossy().to_string()),
    );
    Value::Object(categories)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn get_config_has_three_keys() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let v = get_config(&root);
        assert!(v.get("archive_root").is_some());
        assert!(v.get("kb_root").is_some());
        assert!(v.get("corpus_github").is_some());
    }

    #[test]
    fn get_topics_missing_returns_error_shape() {
        let dir = tempfile::tempdir().expect("tmp");
        let v = get_topics(dir.path());
        assert!(v.get("error").is_some());
    }

    #[test]
    fn get_draft_invalid_path() {
        let dir = tempfile::tempdir().expect("tmp");
        let v = get_draft(dir.path(), "..%2Fsecret");
        assert_eq!(v["error"], "Invalid path");
    }

    #[test]
    fn get_annotation_decodes_percent_encoding() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(corpus.join("annotations").join("ai")).expect("mkdir");
        let ann_path = corpus.join("annotations/ai/my note.json");
        let mut f = fs::File::create(&ann_path).expect("file");
        f.write_all(br#"{"done":true}"#).expect("write");
        // point corpus_root via meili would be heavy — test read_annotation via get_annotation with repo that has meili pointing to corpus
        let mut envf = fs::File::create(dir.path().join("meili.env")).expect("env");
        writeln!(envf, "KNOWLEDGE_CORPUS_DIR={}", corpus.display()).expect("w");
        let q = urlencoding::encode("ai/my note.md");
        let v = get_annotation(dir.path(), &q);
        assert_eq!(v["done"], json!(true));
    }
}
