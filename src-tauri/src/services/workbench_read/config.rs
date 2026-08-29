use std::path::Path;
use std::time::UNIX_EPOCH;

use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

use crate::config::secrets;
use crate::config::settings;

pub fn check_workbench_root(path: &str) -> Value {
    let p = settings::expand_user_path(path);
    if path.trim().is_empty() {
        return json!({ "ok": false, "error": "路径为空" });
    }
    if !p.is_dir() {
        return json!({
            "ok": false,
            "error": format!("目录不存在：{}", p.display())
        });
    }
    let notes_index = p.join("notes").join("index.json");
    let legacy_index = p.join("index.json");
    if !notes_index.is_file() && !legacy_index.is_file() {
        return json!({
            "ok": false,
            "error": format!("未找到 index.json：{}", notes_index.display())
        });
    }
    json!({ "ok": true })
}

pub fn infer_github_user_url(workbench_root: &str) -> Value {
    let path = settings::expand_user_path(workbench_root);
    let (github_user_url, workbench_github_repo_url) =
        crate::config::settings::infer_workbench_github_from_root(&path);
    serde_json::json!({
        "github_user_url": github_user_url,
        "workbench_github_repo_url": workbench_github_repo_url,
    })
}

pub fn get_config(_repo_root: &Path) -> Value {
    let _ = _repo_root;
    let s = settings::load().unwrap_or_default();
    settings::to_config_json(
        &s,
        secrets::has_github_token(),
        secrets::has_meili_key(),
        secrets::has_host_key(),
    )
}

/// Derive topics from sediment-kb repos (+ inbox virtual entry).
pub fn get_topics(_repo_root: &Path) -> Value {
    let _ = _repo_root;
    let rows = match crate::services::sediment_kb::list_repos_for_topics() {
        Ok(rows) => rows,
        Err(e) => return json!({ "error": format!("topics: {e}") }),
    };

    let mut topics_list: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "repo": r.repo,
                "description": r.description,
                "category_id": r.category_id,
                "category_name": r.category_name,
            })
        })
        .collect();
    topics_list.push(json!({ "dir": "inbox", "inbox": true }));

    let mut result = json!({
        "version": 4,
        "source": "sediment-kb",
        "defaultBranch": "main",
        "topics": topics_list,
    });

    if let Ok(cache_path) = crate::config::paths::sediment_kb_repos_path() {
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
    }
    result
}
