//! Settle comment to GitHub knowledge repo (`server.py::_handle_settle`).

use std::path::Path;

use chrono::{Datelike, FixedOffset, Utc};
use serde_json::{json, Value};

use crate::config::meili_env::{corpus_github_string, corpus_root_path};
use crate::integrations::github::{self, decode_contents_payload};
use crate::integrations::search::{build_knowledge_document, MeiliBackend};
use crate::repositories::annotation_paths::annotation_json_path;
use crate::repositories::atomic_json;
use crate::services::annotation::read_annotation_object;
use crate::services::workbench_read::get_topics;

const LAYERS: &[&str] = &["raw", "distilled", "digest", "trace", "diagnose"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleParams {
    pub common_path: String,
    pub comment_id: String,
    pub layer: String,
    pub doc_theme: String,
    pub slug: String,
    pub content: String,
    pub project_dir: String,
    pub target_repo: String,
    pub topic_desc: String,
    pub owner: String,
    pub repo_name: String,
    pub filename: String,
    pub dst_path: String,
    pub dst_url: String,
    pub full_content: String,
}

pub fn valid_slug(slug: &str) -> bool {
    let mut chars = slug.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn valid_doc_theme(doc_theme: &str) -> bool {
    if doc_theme.is_empty() {
        return false;
    }
    if doc_theme == "." {
        return true;
    }
    !doc_theme.contains("..") && !doc_theme.contains('/')
}

pub fn resolve_target_repo(topics: &Value, project_dir: &str) -> Option<(String, String)> {
    let arr = topics.get("topics")?.as_array()?;
    for t in arr {
        let repo = t.get("repo")?.as_str()?;
        let repo_name = repo.split('/').next_back().unwrap_or(repo);
        let dir_match = t.get("dir").and_then(|v| v.as_str()) == Some(project_dir);
        if repo_name == project_dir || dir_match {
            let desc = t
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Some((repo.to_string(), desc));
        }
    }
    None
}

pub fn build_settle_artifacts(
    corpus_github: &str,
    common_path: &str,
    slug: &str,
    doc_theme: &str,
    content: &str,
    owner: &str,
    repo_name: &str,
    ts: &str,
    date_str: &str,
) -> (String, String, String, String) {
    let filename = format!("{ts}-{slug}.md");
    let dst_path = if doc_theme == "." {
        filename.clone()
    } else {
        format!("{doc_theme}/{filename}")
    };
    let dst_url = format!("https://github.com/{owner}/{repo_name}/blob/main/{dst_path}");
    let src_url = format!("{corpus_github}/raw/{common_path}");
    let full_content = format!(
        "> 来源：[Entry]({src_url})\n> 沉淀时间：{date_str}\n\n{content}"
    );
    (filename, dst_path, dst_url, full_content)
}

pub fn index_row(doc_theme: &str, dst_url: &str) -> String {
    format!("| {doc_theme} | （待补充） | {dst_url} |")
}

pub fn append_index_content(existing: &str, row: &str) -> String {
    let mut idx = existing.to_string();
    if !idx.ends_with('\n') {
        idx.push('\n');
    }
    idx.push_str(row);
    idx.push('\n');
    idx
}

pub fn new_index_content(repo_name: &str, row: &str) -> String {
    format!(
        "# {repo_name} 知识索引\n\n\
         | 主题 | 一句话描述 | GitHub URL |\n\
         |------|-----------|------------|\n\
         {row}\n"
    )
}

pub fn append_link(ann: &mut Value, dst_url: &str) {
    if let Some(obj) = ann.as_object_mut() {
        let links = obj.entry("links".to_string()).or_insert_with(|| json!([]));
        if let Some(arr) = links.as_array_mut() {
            arr.push(json!({ "url": dst_url }));
        }
    }
}

pub fn remove_comment(ann: &mut Value, layer: &str, comment_id: &str) {
    let Some(obj) = ann.as_object_mut() else {
        return;
    };
    let Some(layer_val) = obj.get_mut(layer) else {
        return;
    };
    let Some(layer_obj) = layer_val.as_object_mut() else {
        return;
    };
    if let Some(comments) = layer_obj.get_mut("comments").and_then(|v| v.as_array_mut()) {
        comments.retain(|c| c.get("id").and_then(|v| v.as_str()) != Some(comment_id));
        if comments.is_empty() {
            layer_obj.remove("comments");
        }
    }
    if layer_obj.is_empty() {
        obj.remove(layer);
    }
}

pub fn validate_payload(payload: &Value) -> Result<SettleParams, Value> {
    let common_path = payload
        .get("common_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let comment_id = payload
        .get("comment_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let layer = payload
        .get("layer")
        .and_then(|v| v.as_str())
        .unwrap_or("raw")
        .trim();
    let doc_theme = payload
        .get("doc_theme")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let slug = payload.get("slug").and_then(|v| v.as_str()).unwrap_or("").trim();
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if common_path.is_empty() || common_path.contains("..") {
        return Err(json!({ "error": "Invalid common_path", "_status": 400 }));
    }
    if comment_id.is_empty() {
        return Err(json!({ "error": "comment_id required", "_status": 400 }));
    }
    if !LAYERS.contains(&layer) {
        return Err(json!({
            "error": format!("Invalid layer: {layer}"),
            "_status": 400
        }));
    }
    if !valid_doc_theme(doc_theme) {
        return Err(json!({ "error": "Invalid doc_theme", "_status": 400 }));
    }
    if !valid_slug(slug) {
        return Err(json!({
            "error": "slug must be lowercase letters/digits/hyphens, not starting with hyphen",
            "_status": 400
        }));
    }
    if content.is_empty() {
        return Err(json!({ "error": "content required", "_status": 400 }));
    }

    let project_dir = common_path.split('/').next().unwrap_or("").to_string();
    if project_dir.is_empty() {
        return Err(json!({ "error": "Invalid common_path", "_status": 400 }));
    }

    Ok(SettleParams {
        common_path: common_path.to_string(),
        comment_id: comment_id.to_string(),
        layer: layer.to_string(),
        doc_theme: doc_theme.to_string(),
        slug: slug.to_string(),
        content: content.to_string(),
        project_dir,
        target_repo: String::new(),
        topic_desc: String::new(),
        owner: String::new(),
        repo_name: String::new(),
        filename: String::new(),
        dst_path: String::new(),
        dst_url: String::new(),
        full_content: String::new(),
    })
}

fn fill_repo_and_artifacts(repo_root: &Path, p: &mut SettleParams) -> Result<(), Value> {
    let topics = get_topics(repo_root);
    let (target_repo, topic_desc) = resolve_target_repo(&topics, &p.project_dir).ok_or_else(|| {
        json!({
            "error": format!("No GitHub repo found for project: {}", p.project_dir),
            "_status": 400
        })
    })?;
    let parts: Vec<&str> = target_repo.split('/').collect();
    if parts.len() != 2 {
        return Err(json!({ "error": "invalid repo format", "_status": 500 }));
    }
    let owner = parts[0].to_string();
    let repo_name = parts[1].to_string();

    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    let now = Utc::now().with_timezone(&tz);
    let ts = now.format("%Y%m%d%H%M").to_string();
    let date_str = format!("{}年{}月{}日", now.year(), now.month(), now.day());

    let corpus_github = corpus_github_string(repo_root);
    let (filename, dst_path, dst_url, full_content) = build_settle_artifacts(
        &corpus_github,
        &p.common_path,
        &p.slug,
        &p.doc_theme,
        &p.content,
        &owner,
        &repo_name,
        &ts,
        &date_str,
    );

    p.target_repo = target_repo;
    p.topic_desc = topic_desc;
    p.owner = owner;
    p.repo_name = repo_name;
    p.filename = filename;
    p.dst_path = dst_path;
    p.dst_url = dst_url;
    p.full_content = full_content;
    Ok(())
}

fn meili_upsert_best_effort(repo_root: &Path, p: &SettleParams) -> Option<String> {
    let meili = MeiliBackend::new(repo_root);
    let doc = build_knowledge_document(
        &p.target_repo,
        &p.dst_path,
        &p.full_content,
        &p.topic_desc,
    );
    match meili.upsert_knowledge_documents(&[doc]) {
        Ok(()) => None,
        Err(e) => Some(format!("Meilisearch upsert 失败：{e}")),
    }
}

fn update_annotation_link(
    corpus: &Path,
    common_path: &str,
    dst_url: &str,
) -> Result<(), String> {
    let Some(target) = annotation_json_path(corpus, common_path) else {
        return Err("invalid common_path".into());
    };
    let mut ann = read_annotation_object(corpus, common_path);
    append_link(&mut ann, dst_url);
    atomic_json::write_json(&target, &ann).map_err(|e| e.to_string())
}

fn delete_comment_from_annotation(
    corpus: &Path,
    common_path: &str,
    layer: &str,
    comment_id: &str,
) -> Result<(), String> {
    let Some(target) = annotation_json_path(corpus, common_path) else {
        return Err("invalid common_path".into());
    };
    let mut ann = read_annotation_object(corpus, common_path);
    remove_comment(&mut ann, layer, comment_id);
    atomic_json::write_json(&target, &ann).map_err(|e| e.to_string())
}

fn update_remote_index(p: &SettleParams) -> Option<String> {
    let row = index_row(&p.doc_theme, &p.dst_url);
    match github::get_contents(&p.owner, &p.repo_name, "_index.md", None) {
        Ok(info) => {
            let content = match decode_contents_payload(&info) {
                Ok(c) => c,
                Err(e) => return Some(format!("更新 _index.md 失败：{}", e.message)),
            };
            let sha = info.get("sha").and_then(|v| v.as_str()).unwrap_or("");
            if sha.is_empty() {
                return Some("更新 _index.md 失败：missing sha".into());
            }
            let idx_content = append_index_content(&content, &row);
            let msg = format!("settle: update _index.md for {}", p.doc_theme);
            if let Err(e) = github::put_contents(
                &p.owner,
                &p.repo_name,
                "_index.md",
                &msg,
                &idx_content,
                Some(sha),
            ) {
                return Some(format!("更新 _index.md 失败：{}", e.message));
            }
            None
        }
        Err(e) if e.status == Some(404) => {
            let idx_content = new_index_content(&p.repo_name, &row);
            let msg = "settle: create _index.md".to_string();
            if let Err(e) = github::put_contents(
                &p.owner,
                &p.repo_name,
                "_index.md",
                &msg,
                &idx_content,
                None,
            ) {
                return Some(format!("创建 _index.md 失败：{}", e.message));
            }
            None
        }
        Err(e) => Some(format!("更新 _index.md 失败：{}", e.message)),
    }
}

pub fn settle_entry(repo_root: &Path, payload: &Value) -> Value {
    let mut p = match validate_payload(payload) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if let Err(v) = fill_repo_and_artifacts(repo_root, &mut p) {
        return v;
    }

    let commit_msg = format!("settle: add {}/{}", p.doc_theme, p.filename);
    if let Err(e) = github::put_contents(
        &p.owner,
        &p.repo_name,
        &p.dst_path,
        &commit_msg,
        &p.full_content,
        None,
    ) {
        return json!({
            "error": format!("写入目标仓库失败：{}", e.message),
            "_status": 500
        });
    }

    let mut warns: Vec<String> = Vec::new();
    if let Some(w) = meili_upsert_best_effort(repo_root, &p) {
        warns.push(w);
    }

    let corpus = corpus_root_path(repo_root);
    if let Err(e) = update_annotation_link(&corpus, &p.common_path, &p.dst_url) {
        warns.push(format!("更新 annotation.links 失败：{e}"));
    }

    if let Some(w) = update_remote_index(&p) {
        warns.push(w);
    }

    if let Err(e) = delete_comment_from_annotation(
        &corpus,
        &p.common_path,
        &p.layer,
        &p.comment_id,
    ) {
        warns.push(format!("删除 comment 失败：{e}"));
    }

    let mut resp = json!({ "ok": true, "url": p.dst_url });
    if !warns.is_empty() {
        resp["warn"] = json!(warns);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_repo_list<F: FnOnce(&Path)>(repos_json: &str, f: F) {
        let _g = LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        let cache = dir.path().join("cache");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("repo-list.json"), repos_json).expect("repo-list");
        settings::write_test_config(dir.path(), &corpus, None);
        let cfg = settings::load().expect("load");
        let mut s = cfg;
        s.cache_dir = cache;
        settings::save(&s).expect("save");
        f(dir.path());
        settings::set_test_config_dir(None);
    }

    #[test]
    fn valid_slug_rejects_leading_hyphen() {
        assert!(!valid_slug("-abc"));
        assert!(valid_slug("a-b-c"));
        assert!(valid_slug("9slug"));
    }

    #[test]
    fn validate_rejects_invalid_layer() {
        let v = validate_payload(&json!({
            "common_path": "proj/x.md",
            "comment_id": "c1",
            "layer": "invalid",
            "doc_theme": ".",
            "slug": "note",
            "content": "body"
        }))
        .expect_err("layer");
        assert_eq!(v["_status"], 400);
    }

    #[test]
    fn validate_rejects_unknown_project() {
        with_repo_list(r#"{"repos":[]}"#, |root| {
            let v = settle_entry(
                root,
                &json!({
                    "common_path": "unknown/x.md",
                    "comment_id": "c1",
                    "layer": "raw",
                    "doc_theme": ".",
                    "slug": "note",
                    "content": "body"
                }),
            );
            assert_eq!(v["_status"], 400);
            assert!(v["error"]
                .as_str()
                .unwrap_or("")
                .contains("No GitHub repo"));
        });
    }

    #[test]
    fn resolve_target_repo_by_dir_or_name() {
        let topics = json!({
            "topics": [
                { "repo": "o/my-repo", "dir": "proj", "description": "d1" },
                { "repo": "o/other", "description": "d2" }
            ]
        });
        let (r, desc) = resolve_target_repo(&topics, "proj").expect("proj");
        assert_eq!(r, "o/my-repo");
        assert_eq!(desc, "d1");
        let (r2, _) = resolve_target_repo(&topics, "other").expect("name");
        assert_eq!(r2, "o/other");
    }

    #[test]
    fn build_settle_artifacts_root_theme() {
        let (_, dst_path, url, body) = build_settle_artifacts(
            "https://gh.com/corpus",
            "proj/note.md",
            "slug",
            ".",
            "hello",
            "owner",
            "repo",
            "202501011200",
            "2025年1月1日",
        );
        assert_eq!(dst_path, "202501011200-slug.md");
        assert!(url.contains("blob/main/202501011200-slug.md"));
        assert!(body.contains("proj/note.md"));
        assert!(body.contains("hello"));
    }

    #[test]
    fn append_index_and_new_index() {
        let row = index_row("theme", "https://example.com/u");
        let appended = append_index_content("| a | b | c |\n", &row);
        assert!(appended.contains(&row));
        let created = new_index_content("repo", &row);
        assert!(created.contains("知识索引"));
        assert!(created.contains(&row));
    }

    #[test]
    fn append_link_and_remove_comment() {
        let mut ann = json!({
            "raw": { "comments": [
                { "id": "keep", "text": "a" },
                { "id": "drop", "text": "b" }
            ]}
        });
        append_link(&mut ann, "https://x");
        assert_eq!(ann["links"][0]["url"], "https://x");
        remove_comment(&mut ann, "raw", "drop");
        assert_eq!(ann["raw"]["comments"].as_array().unwrap().len(), 1);
        remove_comment(&mut ann, "raw", "keep");
        assert!(ann.get("raw").is_none());
    }

    #[test]
    fn local_annotation_updated_when_put_would_succeed() {
        with_repo_list(
            r#"{"repos":[{"full_name":"o/proj","name":"proj","type":"KNOWLEDGE_CORPUS","description":""}]}"#,
            |root| {
                let corpus = root.join("corpus");
                fs::create_dir_all(corpus.join("annotations/proj")).expect("ann dir");
                let ann_path = corpus.join("annotations/proj/note.json");
                fs::write(
                    &ann_path,
                    r#"{"raw":{"comments":[{"id":"c1","text":"t"}]}}"#,
                )
                .expect("ann");
                let mut ann = read_annotation_object(&corpus, "proj/note.md");
                append_link(&mut ann, "https://github.com/o/r/blob/main/f.md");
                remove_comment(&mut ann, "raw", "c1");
                atomic_json::write_json(&ann_path, &ann).expect("write");
                let loaded: Value =
                    serde_json::from_str(&fs::read_to_string(&ann_path).unwrap()).unwrap();
                assert_eq!(loaded["links"][0]["url"], "https://github.com/o/r/blob/main/f.md");
                assert!(loaded.get("raw").is_none());
            },
        );
    }
}
