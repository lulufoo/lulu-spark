use super::*;
use std::fs;
use std::path::PathBuf;

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
