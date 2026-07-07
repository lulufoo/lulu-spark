use super::*;
use std::fs;
use std::path::PathBuf;

use crate::test_support::TestSandbox;

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


fn with_sediment_kb_topics_cache<F: FnOnce(&std::path::Path, &std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir");
    f(cfg_dir, wb.as_path());
}

fn with_corpus_repo<F: FnOnce(&std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let wb = sandbox.workbench_knowledge_root();
    setup(cfg_dir, wb.as_path());
    f(cfg_dir);
}

#[test]
fn get_topics_missing_sediment_kb_inits_and_returns_inbox_only() {
    with_sediment_kb_topics_cache(|cfg, corpus| {
        let v = get_topics(corpus);
        assert!(v.get("error").is_none(), "expected init fallback, got {v:?}");
        assert_eq!(v["source"], "sediment-kb");
        let topics = v["topics"].as_array().expect("topics array");
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0]["dir"], "inbox");
        assert_eq!(topics[0]["inbox"], true);
        let _ = cfg;
    });
}

#[test]
fn get_topics_reads_from_sediment_kb_with_category_fields() {
    use crate::services::sediment_kb::{
        add_repo, ensure_uncategorized, set_test_repo_validator, UNCATEGORIZED_ID,
    };

    with_sediment_kb_topics_cache(|cfg, corpus| {
        set_test_repo_validator(Some(|name| Ok(name.to_string())));
        ensure_uncategorized().expect("ensure");
        add_repo("lulufoo/kb-a", None, "Saved KB description").expect("add");
        set_test_repo_validator(None);

        let v = get_topics(corpus);
        assert!(v.get("error").is_none(), "unexpected error: {v:?}");
        assert_eq!(v["source"], "sediment-kb");
        let topics = v["topics"].as_array().expect("topics array");
        let kb = topics
            .iter()
            .find(|t| t.get("repo") == Some(&serde_json::json!("lulufoo/kb-a")))
            .expect("sediment-kb repo topic");
        assert_eq!(kb["description"], "Saved KB description");
        assert_eq!(kb["category_id"], UNCATEGORIZED_ID);
        assert_eq!(kb["category_name"], "未分类");
        assert!(
            !topics
                .iter()
                .any(|t| t.get("repo") == Some(&serde_json::json!("lulufoo/ignored")))
        );
        assert!(topics.iter().any(|t| t.get("inbox") == Some(&serde_json::json!(true))));
    });
}

#[test]
fn load_repos_deserializes_legacy_entry_without_description_field() {
    use crate::config::paths;
    use crate::services::sediment_kb::{ensure_uncategorized, load_repos, UNCATEGORIZED_ID};

    with_sediment_kb_topics_cache(|_, _| {
        ensure_uncategorized().expect("ensure");
        let repos_path = paths::sediment_kb_repos_path().expect("repos path");
        fs::write(
            &repos_path,
            format!(
                r#"{{"version":1,"repos":[{{"full_name":"legacy/repo","category_id":"{UNCATEGORIZED_ID}"}}]}}"#
            ),
        )
        .expect("write legacy repos.json");

        let repos = load_repos().expect("load legacy repos");
        assert_eq!(repos.repos.len(), 1);
        assert_eq!(repos.repos[0].full_name, "legacy/repo");
        assert_eq!(repos.repos[0].description, "");
    });
}

#[test]
fn description_source_constraints_hold() {
    crate::services::sediment_kb::validate_description_source_constraints();
}

#[test]
fn get_topics_empty_repos_returns_inbox_only() {
    use crate::services::sediment_kb::ensure_uncategorized;

    with_sediment_kb_topics_cache(|_, corpus| {
        ensure_uncategorized().expect("ensure");
        let v = get_topics(corpus);
        assert!(v.get("error").is_none(), "unexpected error: {v:?}");
        let topics = v["topics"].as_array().expect("topics array");
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0]["inbox"], true);
    });
}

#[test]
fn list_repos_for_topics_resolves_uncategorized_name() {
    use crate::services::sediment_kb::{
        add_repo, ensure_uncategorized, list_repos_for_topics, set_test_repo_validator,
        UNCATEGORIZED_ID,
    };

    with_sediment_kb_topics_cache(|_, _| {
        set_test_repo_validator(Some(|name| Ok(name.to_string())));
        ensure_uncategorized().expect("ensure");
        add_repo("acme/demo", None, "").expect("add");
        set_test_repo_validator(None);

        let rows = list_repos_for_topics().expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].repo, "acme/demo");
        assert_eq!(rows[0].category_id, UNCATEGORIZED_ID);
        assert_eq!(rows[0].category_name, "未分类");
    });
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
    with_corpus_repo(
        |_, corpus| {
            fs::create_dir_all(corpus).expect("mkdir");
            fs::write(corpus.join("index.json"), br#"{"entries":[]}"#).expect("write");
        },
        |cfg_dir| {
            let v = get_corpus_index(cfg_dir);
            assert_eq!(v["entries"], json!([]));
        },
    );
}

#[test]
fn get_corpus_file_rejects_traversal() {
    with_corpus_repo(
        |_, corpus| {
            fs::create_dir_all(corpus.join("raw")).expect("mkdir");
        },
        |cfg_dir| {
            let v = get_corpus_file(cfg_dir, "raw", "../index.json");
            assert_eq!(v["error"], "Invalid path");
        },
    );
}

#[test]
fn get_annotations_summary_includes_resolved_tags() {
    use crate::repositories::annotation_paths::annotation_json_path;
    use crate::repositories::atomic_json;
    use crate::test_support::with_sandbox_corpus;

    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/tags.md";
        fs::write(
            corpus.join("index.json"),
            serde_json::to_string(&serde_json::json!({
                "entries": { "e1": { "common_path": cp } }
            }))
            .unwrap(),
        )
        .expect("index");
        let ann_path = annotation_json_path(&corpus, cp).expect("path");
        if let Some(parent) = ann_path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        atomic_json::write_json(
            &ann_path,
            &serde_json::json!({ "tag_keys": ["knownkey123456", "ghostkey123456"] }),
        )
        .expect("ann");
        let mut reg = crate::services::tags_registry::empty_registry();
        reg["keys"]["knownkey123456"] = serde_json::json!({ "value": "Known", "refs": 1 });
        assert!(crate::services::tags_registry::save_registry(&corpus, &reg).is_none());

        let summary = get_annotations_summary(dir);
        let entry = summary[cp].as_object().expect("entry");
        let tags = entry["tags"].as_array().expect("tags");
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().any(|t| t["key"] == "knownkey123456" && t["value"] == "Known"));
        assert!(
            tags.iter()
                .any(|t| t["key"] == "ghostkey123456" && t["unknown"] == true)
        );
    });
}

#[test]
fn get_corpus_file_returns_content() {
    with_corpus_repo(
        |_, corpus| {
            let raw = corpus.join("raw").join("a.md");
            fs::create_dir_all(raw.parent().unwrap()).expect("mkdir");
            fs::write(&raw, b"hello").expect("write");
        },
        |cfg_dir| {
            let v = get_corpus_file(cfg_dir, "raw", "a.md");
            assert_eq!(v["content"], "hello");
        },
    );
}

#[test]
fn get_corpus_asset_returns_png_bytes() {
    with_corpus_repo(
        |_, corpus| {
            let png = corpus.join("raw").join("ai").join("note.png");
            fs::create_dir_all(png.parent().unwrap()).expect("mkdir");
            fs::write(&png, b"\x89PNG\r\n").expect("write");
            fs::write(corpus.join("raw").join("ai").join("note.md"), b"# x").expect("md");
        },
        |cfg_dir| {
            let v = get_corpus_asset(cfg_dir, "raw", "ai/note.md", "note.png");
            assert!(v.get("data_b64").and_then(|x| x.as_str()).is_some());
            assert_eq!(v["mime_type"], "image/png");
        },
    );
}

#[test]
fn get_corpus_asset_rejects_traversal() {
    with_corpus_repo(
        |_, corpus| {
            fs::create_dir_all(corpus.join("raw")).expect("mkdir");
        },
        |cfg_dir| {
            let v = get_corpus_asset(cfg_dir, "raw", "ai/note.md", "../index.json");
            assert_eq!(v["error"], "Invalid path");
        },
    );
}

#[test]
fn get_corpus_asset_rejects_non_whitelist_ext() {
    with_corpus_repo(
        |_, corpus| {
            let svg = corpus.join("raw").join("x.svg");
            fs::create_dir_all(svg.parent().unwrap()).expect("mkdir");
            fs::write(&svg, b"<svg").expect("write");
            fs::write(corpus.join("raw").join("x.md"), b"#").expect("md");
        },
        |cfg_dir| {
            let v = get_corpus_asset(cfg_dir, "raw", "x.md", "x.svg");
            assert_eq!(v["error"], "Unsupported media type");
        },
    );
}

#[test]
fn get_corpus_catalog_latest_per_topic_picks_newest() {
    let id_old = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let id_new = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let id_pg = "cccccccccccccccccccccccccccccccc";
    with_corpus_repo(
        move |_, corpus| {
            fs::create_dir_all(corpus.join("digest/ai")).expect("mkdir");
            fs::create_dir_all(corpus.join("digest/personal-growth")).expect("mkdir");
            fs::write(corpus.join("digest/ai/old.md"), b"old").expect("write");
            fs::write(corpus.join("digest/ai/new.md"), b"new").expect("write");
            fs::write(corpus.join("digest/personal-growth/speech.md"), b"speech").expect("write");
            let index = json!({
                "entries": {
                    id_old: {
                        "common_path": "ai/old.md",
                        "created_at": "202606010001",
                        "layers": ["digest"]
                    },
                    id_new: {
                        "common_path": "ai/new.md",
                        "created_at": "202606190004",
                        "layers": ["digest"]
                    },
                    id_pg: {
                        "common_path": "personal-growth/speech.md",
                        "created_at": "202606190947",
                        "layers": ["digest"]
                    },
                    "not-hex-id": {
                        "common_path": "ai/skip.md",
                        "created_at": "202606999999",
                        "layers": ["digest"]
                    },
                    "dddddddddddddddddddddddddddddddd": {
                        "common_path": "ai/no-digest.md",
                        "created_at": "202606999999",
                        "layers": ["raw"]
                    }
                }
            });
            fs::write(corpus.join("index.json"), index.to_string()).expect("write index");
        },
        move |cfg_dir| {
            let v = get_corpus_catalog_latest_per_topic(cfg_dir);
            let items = v["items"].as_array().expect("items");
            assert_eq!(items.len(), 2);
            let ai = items.iter().find(|i| i["topic"] == "ai").expect("ai topic");
            assert_eq!(ai["id"], id_new);
            assert_eq!(ai["created_at"], "202606190004");
            assert!(ai.get("common_path").is_none());
            let pg = items.iter().find(|i| i["topic"] == "personal-growth").expect("pg");
            assert_eq!(pg["id"], id_pg);
        },
    );
}

#[test]
fn get_corpus_files_by_ids_batch() {
    let id_ok = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    with_corpus_repo(
        move |_, corpus| {
            fs::create_dir_all(corpus.join("digest/ai")).expect("mkdir");
            fs::write(corpus.join("digest/ai/note.md"), b"# digest body").expect("write");
            let index = json!({
                "entries": {
                    id_ok: {
                        "common_path": "ai/note.md",
                        "created_at": "202606190004",
                        "layers": ["digest"]
                    }
                }
            });
            fs::write(corpus.join("index.json"), index.to_string()).expect("write index");
        },
        move |cfg_dir| {
            let v = get_corpus_files_by_ids(
                cfg_dir,
                &[
                    id_ok.to_string(),
                    "ffffffffffffffffffffffffffffffff".to_string(),
                    "bad".to_string(),
                ],
            );
            let items = v["items"].as_array().expect("items");
            assert_eq!(items.len(), 3);
            assert_eq!(items[0]["ok"], true);
            assert_eq!(items[0]["content"], "# digest body");
            assert_eq!(items[1]["ok"], false);
            assert_eq!(items[2]["ok"], false);
        },
    );
}

#[test]
fn get_corpus_files_by_ids_rejects_empty() {
    with_corpus_repo(
        |_, corpus| {
            fs::create_dir_all(corpus).expect("mkdir");
            fs::write(corpus.join("index.json"), br#"{"entries":{}}"#).expect("write");
        },
        |cfg_dir| {
            let v = get_corpus_files_by_ids(cfg_dir, &[]);
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn get_annotation_decodes_percent_encoding() {
    with_corpus_repo(
        |_, corpus| {
            fs::create_dir_all(corpus.join("annotations").join("ai")).expect("mkdir");
            let ann_path = corpus.join("annotations/ai/note.json");
            fs::write(&ann_path, br#"{"done":true}"#).expect("write");
        },
        |cfg_dir| {
            let v = get_annotation(cfg_dir, "ai/note.md");
            assert_eq!(v["done"], json!(true));
        },
    );
}
