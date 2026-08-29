use super::*;
use crate::test_support::TestSandbox;
use std::fs;
use std::path::Path;

fn with_repo_list<F: FnOnce(&Path)>(repos_json: &str, f: F) {
    let sandbox = TestSandbox::new();
    let cache = sandbox.cache_dir();
    fs::write(cache.join("repo-list.json"), repos_json).expect("repo-list");
    f(sandbox.config_dir());
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
        "https://gh.com/notes",
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
        r#"{"repos":[{"full_name":"o/proj","name":"proj","type":"knowledge","description":""}]}"#,
        |root| {
            let wb = crate::config::settings::load()
                .expect("load")
                .workbench_root;
            fs::create_dir_all(wb.join("annotations/proj")).expect("ann dir");
            let ann_path = wb.join("annotations/proj/note.json");
            fs::write(
                &ann_path,
                r#"{"raw":{"comments":[{"id":"c1","text":"t"}]}}"#,
            )
            .expect("ann");
            let mut ann = read_annotation_object(&wb, "proj/note.md");
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
