//! Corpus entry delete / move-project (local FS + index.json only).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::config::paths;
use crate::repositories::annotation_paths::annotation_json_path;
const LAYERS: &[&str] = &["raw", "distilled", "trace", "digest", "diagnose"];

fn workbench_knowledge_root() -> Result<PathBuf, Value> {
    paths::workbench_knowledge_root().map_err(|e| json!({ "error": format!("{e:?}") }))
}

fn load_index(corpus: &Path) -> Result<(PathBuf, Map<String, Value>), Value> {
    let index_path = corpus.join("index.json");
    if !index_path.is_file() {
        return Err(json!({ "error": "index.json not found", "_status": 404 }));
    }
    let text = fs::read_to_string(&index_path).map_err(|e| json!({ "error": e.to_string() }))?;
    let data: Value =
        serde_json::from_str(&text).map_err(|e| json!({ "error": e.to_string() }))?;
    let entries = if let Some(obj) = data.get("entries").and_then(|v| v.as_object()) {
        obj.clone()
    } else if let Some(obj) = data.as_object() {
        obj.clone()
    } else {
        return Err(json!({ "error": "invalid index.json", "_status": 500 }));
    };
    Ok((index_path, entries))
}

fn save_index(index_path: &Path, entries: &Map<String, Value>) -> Result<(), Value> {
    let data = json!({ "entries": entries });
    let text = serde_json::to_string_pretty(&data).map_err(|e| json!({ "error": e.to_string() }))?;
    let tmp = index_path.with_extension("json.tmp");
    fs::write(&tmp, &text).map_err(|e| json!({ "error": e.to_string() }))?;
    fs::rename(&tmp, index_path).map_err(|e| json!({ "error": e.to_string() }))?;
    Ok(())
}

fn valid_projects() -> HashSet<String> {
    let Ok(repo_root) = crate::config::paths::repo_root() else {
        return HashSet::new();
    };
    let topics = crate::services::workbench_read::get_topics(&repo_root);
    let mut set = HashSet::new();
    if let Some(arr) = topics.get("topics").and_then(|v| v.as_array()) {
        for t in arr {
            if let Some(d) = t.get("dir").and_then(|v| v.as_str()) {
                set.insert(d.to_string());
            } else if let Some(r) = t.get("repo").and_then(|v| v.as_str()) {
                if let Some(name) = r.split('/').next_back() {
                    set.insert(name.to_string());
                }
            }
        }
    }
    set
}

fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn delete_entry(payload: &Value) -> Value {
    let entry_id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !is_valid_entry_id(entry_id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }
    let corpus = match workbench_knowledge_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    let (index_path, mut entries) = match load_index(&corpus) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let Some(entry) = entries.remove(entry_id) else {
        return json!({ "error": "Entry not found", "_status": 404 });
    };
    let common_path = entry.get("common_path").and_then(|v| v.as_str()).unwrap_or("");
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid common_path", "_status": 400 });
    }
    let mut deleted = Vec::new();
    for layer in LAYERS {
        let target = corpus.join(layer).join(common_path);
        if !target.starts_with(&corpus) {
            continue;
        }
        if target.is_file() {
            let _ = fs::remove_file(&target);
            deleted.push(format!("{layer}/{common_path}"));
        }
    }
    if let Some(ann) = annotation_json_path(&corpus, common_path) {
        if ann.is_file() {
            let _ = fs::remove_file(&ann);
            deleted.push(format!("annotations/{}.json", &common_path[..common_path.len().saturating_sub(3)]));
        }
    }
    if let Err(v) = save_index(&index_path, &entries) {
        return v;
    }
    json!({ "ok": true, "deleted": deleted })
}

pub fn move_entry_project(payload: &Value) -> Value {
    let entry_id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("").trim();
    let new_project = payload
        .get("new_project")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if !is_valid_entry_id(entry_id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }
    if new_project.is_empty() || new_project.contains("..") || new_project.contains('/') {
        return json!({ "error": "Invalid new_project", "_status": 400 });
    }
    let valid = valid_projects();
    if !valid.is_empty() && !valid.contains(new_project) {
        return json!({ "error": format!("Unknown project: {new_project}"), "_status": 400 });
    }
    let corpus = match workbench_knowledge_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    let (index_path, mut entries) = match load_index(&corpus) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let Some(entry_val) = entries.get(entry_id).cloned() else {
        return json!({ "error": "Entry not found", "_status": 404 });
    };
    let mut entry = entry_val.as_object().cloned().unwrap_or_default();
    let old_cp = entry.get("common_path").and_then(|v| v.as_str()).unwrap_or("");
    if old_cp.is_empty() || old_cp.contains("..") {
        return json!({ "error": "Invalid common_path", "_status": 400 });
    }
    let mut parts: Vec<&str> = old_cp.split('/').collect();
    if parts.is_empty() {
        return json!({ "error": "Invalid common_path", "_status": 400 });
    }
    parts[0] = new_project;
    let new_cp = parts.join("/");
    if old_cp == new_cp {
        return json!({ "error": "Already in this project", "_status": 400 });
    }

    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    for layer in LAYERS {
        let src = corpus.join(layer).join(old_cp);
        let dst = corpus.join(layer).join(&new_cp);
        if src.exists() {
            moves.push((src, dst));
        }
    }
    if let Some(ann_src) = annotation_json_path(&corpus, old_cp) {
        if ann_src.exists() {
            let new_ann_rel = if new_cp.ends_with(".md") {
                format!("{}.json", &new_cp[..new_cp.len() - 3])
            } else {
                format!("{new_cp}.json")
            };
            let ann_dst = corpus.join("annotations").join(&new_ann_rel);
            moves.push((ann_src, ann_dst));
        }
    }
    let old_zh = entry
        .get("translations")
        .and_then(|v| v.get("zh"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut new_zh = None;
    if let Some(ref old_zh) = old_zh {
        let mut zh_parts: Vec<&str> = old_zh.split('/').collect();
        zh_parts[0] = new_project;
        if zh_parts.len() >= 2 && parts.len() >= 2 {
            zh_parts[1] = parts[1];
        }
        new_zh = Some(zh_parts.join("/"));
        let zh_src = corpus.join("raw").join(zh_parts.join("/"));
        let zh_dst = corpus.join("raw").join(new_zh.as_ref().unwrap());
        if zh_src.exists() {
            moves.push((zh_src, zh_dst));
        }
    }

    let mut completed = Vec::new();
    for (src, dst) in &moves {
        if let Some(parent) = dst.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                for (s, d) in completed.iter().rev() {
                    let _ = fs::rename(d, s);
                }
                return json!({ "error": format!("Move failed, rolled back: {e}"), "_status": 500 });
            }
        }
        if let Err(e) = fs::rename(src, dst) {
            for (s, d) in completed.iter().rev() {
                let _ = fs::rename(d, s);
            }
            return json!({ "error": format!("Move failed, rolled back: {e}"), "_status": 500 });
        }
        completed.push((src.clone(), dst.clone()));
    }

    for (src, dst) in &completed {
        if dst.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(mut content) = fs::read_to_string(dst) {
            for layer in LAYERS {
                let old_ref = format!("{layer}/{old_cp}");
                let new_ref = format!("{layer}/{new_cp}");
                content = content.replace(&old_ref, &new_ref);
                content = content.replace(&format!("`{old_ref}`"), &format!("`{new_ref}`"));
            }
            if let (Some(ref oz), Some(ref nz)) = (old_zh.as_ref(), new_zh.as_ref()) {
                content = content.replace(&format!("raw/{oz}"), &format!("raw/{nz}"));
            }
            let _ = fs::write(dst, content);
        }
        let _ = src;
    }

    entry.insert("common_path".into(), json!(new_cp));
    if let Some(nz) = new_zh {
        let mut tr = entry
            .get("translations")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        tr.insert("zh".into(), json!(nz));
        entry.insert("translations".into(), Value::Object(tr));
    }
    entries.insert(entry_id.to_string(), Value::Object(entry));
    if let Err(v) = save_index(&index_path, &entries) {
        return v;
    }
    json!({ "ok": true, "new_common_path": new_cp })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings;
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn delete_entry_removes_file_and_index() {
        let _g = LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(corpus.join("raw/proj")).expect("mkdir");
        fs::write(corpus.join("raw/proj/a.md"), "# x").expect("w");
        let id = "a".repeat(32);
        let index = json!({
            "entries": {
                id.clone(): { "common_path": "proj/a.md" }
            }
        });
        fs::write(
            corpus.join("index.json"),
            serde_json::to_string_pretty(&index).unwrap(),
        )
        .expect("idx");
        settings::write_test_config(dir.path(), &corpus, None);
        let v = delete_entry(&json!({ "id": id }));
        assert_eq!(v["ok"], true);
        assert!(!corpus.join("raw/proj/a.md").exists());
        settings::set_test_config_dir(None);
    }
}
