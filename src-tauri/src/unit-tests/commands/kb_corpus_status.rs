use serde_json::json;

use crate::commands::read::kb_corpus_status_json;
use crate::test_support::with_test_config_dir;

use std::fs;

fn with_repo_list_cache<F: FnOnce()>(repos_json: &str, f: F) {
    with_test_config_dir(|cfg| {
        let cache = cfg.join("cache");
        fs::create_dir_all(&cache).expect("mkdir cache");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"cache_dir = "{}""#, cache.display()),
        )
        .expect("write config");
        fs::write(cache.join("repo-list.json"), repos_json).expect("write repo-list");
        f();
    });
}

#[test]
fn local_http_kb_corpus_status_deprecates_knowledge_corpus_branch() {
    with_repo_list_cache(
        r#"{"repos":[{"full_name":"acme/old","name":"old","type":"KNOWLEDGE_CORPUS","description":"legacy"}]}"#,
        || {
            let v = kb_corpus_status_json(Some("KNOWLEDGE_CORPUS")).expect("ok");
            assert_eq!(v["deprecated"], json!(true));
            assert_eq!(v["repos"], json!([]));
        },
    );
}

#[test]
fn local_http_kb_corpus_status_defaults_to_deprecated_knowledge_corpus() {
    with_repo_list_cache(
        r#"{"repos":[{"full_name":"acme/old","name":"old","type":"KNOWLEDGE_CORPUS","description":""}]}"#,
        || {
            let v = kb_corpus_status_json(None).expect("ok");
            assert_eq!(v["deprecated"], json!(true));
            assert_eq!(v["repos"], json!([]));
        },
    );
}

#[test]
fn local_http_kb_corpus_status_keeps_workbench_knowledge_branch() {
    with_repo_list_cache(
        r#"{"repos":[{"full_name":"user/wb-kb","name":"wb-kb","type":"WORKBENCH_KNOWLEDGE","description":"wb"}]}"#,
        || {
            let v = kb_corpus_status_json(Some("WORKBENCH_KNOWLEDGE")).expect("ok");
            assert!(v.get("deprecated").is_none());
            let repos = v["repos"].as_array().expect("repos array");
            assert_eq!(repos.len(), 1);
            assert_eq!(repos[0]["full_name"], "user/wb-kb");
        },
    );
}
