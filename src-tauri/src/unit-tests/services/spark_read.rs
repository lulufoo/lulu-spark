use super::*;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

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
    assert!(v.get("spark_root").is_some());
    assert!(v.get("knowledge_root").is_some());
    assert!(v.get("github_user_url").is_some());
    assert!(v.get("spark_github_repo_url").is_some());
    assert!(v.get("assistant_engine").is_some());
    assert!(v.get("cache_dir").is_some());
    assert!(v.get("has_github_token").is_some());
    assert!(v.get("has_host_key").is_some());
    assert!(v.get("has_cursor_key").is_none());
    assert!(v.get("llm").is_some());
    assert!(v.get("api_key").is_none());
    assert!(v.get("api_key_host").is_none());
    assert!(v.get("api_key_cursor").is_none());
    assert!(v["llm"].get("api_key").is_none());
}


fn with_sediment_kb_topics_cache<F: FnOnce(&std::path::Path, &std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let wb = sandbox.spark_root();
    fs::create_dir_all(&wb).expect("mkdir");
    f(cfg_dir, wb.as_path());
}

fn with_notes_repo<F: FnOnce(&std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let notes = sandbox.spark_root().join("notes");
    fs::create_dir_all(&notes).expect("notes");
    setup(cfg_dir, notes.as_path());
    f(cfg_dir);
}

#[test]
fn get_topics_missing_sediment_kb_inits_and_returns_inbox_only() {
    with_sediment_kb_topics_cache(|cfg, wb| {
        let v = get_topics(wb);
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

    with_sediment_kb_topics_cache(|cfg, wb| {
        set_test_repo_validator(Some(|name| Ok(name.to_string())));
        ensure_uncategorized().expect("ensure");
        add_repo("lulufoo/kb-a", None, "Saved KB description").expect("add");
        set_test_repo_validator(None);

        let v = get_topics(wb);
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

    with_sediment_kb_topics_cache(|_, wb| {
        ensure_uncategorized().expect("ensure");
        let v = get_topics(wb);
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

fn init_repo_with_github_origin(dir: &std::path::Path, origin: &str) {
    assert!(crate::integrations::git::exec(dir, &["init"])
        .expect("init")
        .success);
    assert!(crate::integrations::git::exec(dir, &["remote", "add", "origin", origin])
        .expect("remote add")
        .success);
}

#[test]
fn infer_github_user_url_reads_ssh_origin() {
    let dir = tempfile::tempdir().expect("tmp");
    init_repo_with_github_origin(
        dir.path(),
        "git@github.com:lulufoo/lulu-workbench-knowledge.git",
    );
    let v = infer_github_user_url(dir.path().to_str().unwrap());
    assert_eq!(v["github_user_url"], "https://github.com/lulufoo");
    assert_eq!(
        v["spark_github_repo_url"],
        "https://github.com/lulufoo/lulu-workbench-knowledge"
    );
}

#[test]
fn infer_github_user_url_trims_padded_path() {
    let dir = tempfile::tempdir().expect("tmp");
    init_repo_with_github_origin(dir.path(), "git@github.com:lulufoo/notes.git");
    let padded = format!("  {}  ", dir.path().display());
    let v = infer_github_user_url(&padded);
    assert_eq!(v["github_user_url"], "https://github.com/lulufoo");
    assert_eq!(
        v["spark_github_repo_url"],
        "https://github.com/lulufoo/notes"
    );
}

#[test]
fn check_spark_root_requires_dir_and_index() {
    let dir = tempfile::tempdir().expect("tmp");
    let missing = check_spark_root("/no/such/spark");
    assert_eq!(missing["ok"], false);

    let no_index_dir = dir.path().join("empty");
    fs::create_dir_all(&no_index_dir).expect("mkdir");
    let no_index = check_spark_root(no_index_dir.to_str().unwrap());
    assert_eq!(no_index["ok"], false);
    assert!(no_index["error"].as_str().unwrap().contains("index.json"));

    let store = dir.path().join("store");
    fs::create_dir_all(&store).expect("mkdir");
    fs::write(store.join("index.json"), br#"{"entries":[]}"#).expect("write");
    let ok = check_spark_root(store.to_str().unwrap());
    assert_eq!(ok["ok"], true);
}

#[test]
fn get_notes_index_reads_json() {
    with_notes_repo(
        |_, notes| {
            fs::create_dir_all(notes).expect("mkdir");
            fs::write(notes.join("index.json"), br#"{"entries":[]}"#).expect("write");
        },
        |cfg_dir| {
            let v = get_notes_index(cfg_dir);
            assert_eq!(v["entries"], json!([]));
        },
    );
}

#[test]
fn get_notes_file_rejects_traversal() {
    with_notes_repo(
        |_, notes| {
            fs::create_dir_all(notes.join("raw")).expect("mkdir");
        },
        |cfg_dir| {
            let v = get_notes_file(cfg_dir, "raw", "../index.json");
            assert_eq!(v["error"], "Invalid path");
        },
    );
}

#[test]
fn get_annotations_summary_includes_resolved_tags() {
    use crate::repositories::annotation_paths::annotation_json_path;
    use crate::repositories::atomic_json;
    use crate::test_support::with_sandbox_notes;

    with_sandbox_notes(true, |dir, notes| {
        let cp = "ai/tags.md";
        fs::write(
            notes.join("index.json"),
            serde_json::to_string(&serde_json::json!({
                "entries": { "e1": { "common_path": cp } }
            }))
            .unwrap(),
        )
        .expect("index");
        let ann_path = annotation_json_path(&notes, cp).expect("path");
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
        assert!(crate::services::tags_registry::save_registry(&notes, &reg).is_none());

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
fn get_notes_file_reads_abs_path_under_knowledge_root() {
    let sandbox = TestSandbox::new();
    let file = sandbox.knowledge_root().join("demo").join("doc.md");
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, b"kb body").expect("write");
    let v = get_notes_file(
        sandbox.config_dir(),
        "raw",
        &file.to_string_lossy(),
    );
    assert_eq!(v["content"], "kb body", "{v}");
}

#[test]
fn get_notes_file_rejects_abs_path_outside_notes_and_knowledge() {
    let sandbox = TestSandbox::new();
    fs::create_dir_all(sandbox.spark_root().join("notes")).expect("notes");
    let file = sandbox.cache_dir().join("outside.md");
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, b"nope").expect("write");
    let v = get_notes_file(
        sandbox.config_dir(),
        "raw",
        &file.to_string_lossy(),
    );
    assert_eq!(v["error"], "Path traversal not allowed", "{v}");
}

#[test]
fn get_notes_file_returns_content() {
    with_notes_repo(
        |_, notes| {
            let raw = notes.join("raw").join("a.md");
            fs::create_dir_all(raw.parent().unwrap()).expect("mkdir");
            fs::write(&raw, b"hello").expect("write");
        },
        |cfg_dir| {
            let v = get_notes_file(cfg_dir, "raw", "a.md");
            assert_eq!(v["content"], "hello");
        },
    );
}

#[test]
fn get_notes_asset_returns_png_bytes() {
    with_notes_repo(
        |_, notes| {
            let png = notes.join("raw").join("ai").join("note.png");
            fs::create_dir_all(png.parent().unwrap()).expect("mkdir");
            fs::write(&png, b"\x89PNG\r\n").expect("write");
            fs::write(notes.join("raw").join("ai").join("note.md"), b"# x").expect("md");
        },
        |cfg_dir| {
            let v = get_notes_asset(cfg_dir, "raw", "ai/note.md", "note.png");
            assert!(v.get("data_b64").and_then(|x| x.as_str()).is_some());
            assert_eq!(v["mime_type"], "image/png");
        },
    );
}

#[test]
fn get_notes_asset_rejects_traversal() {
    with_notes_repo(
        |_, notes| {
            fs::create_dir_all(notes.join("raw")).expect("mkdir");
        },
        |cfg_dir| {
            let v = get_notes_asset(cfg_dir, "raw", "ai/note.md", "../index.json");
            assert_eq!(v["error"], "Invalid path");
        },
    );
}

#[test]
fn get_notes_asset_rejects_non_whitelist_ext() {
    with_notes_repo(
        |_, notes| {
            let svg = notes.join("raw").join("x.svg");
            fs::create_dir_all(svg.parent().unwrap()).expect("mkdir");
            fs::write(&svg, b"<svg").expect("write");
            fs::write(notes.join("raw").join("x.md"), b"#").expect("md");
        },
        |cfg_dir| {
            let v = get_notes_asset(cfg_dir, "raw", "x.md", "x.svg");
            assert_eq!(v["error"], "Unsupported media type");
        },
    );
}

#[test]
fn list_all_notes_catalogs_picks_newest_including_raw_only() {
    let id_old = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let id_new = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let id_pg = "cccccccccccccccccccccccccccccccc";
    let id_raw = "dddddddddddddddddddddddddddddddd";
    with_notes_repo(
        move |_, notes| {
            fs::create_dir_all(notes.join("digest/ai/notes")).expect("mkdir ai digest");
            fs::create_dir_all(notes.join("digest/personal-growth")).expect("mkdir pg digest");
            fs::write(notes.join("digest/ai/notes/new.md"), b"# newest digest").expect("write");
            fs::write(notes.join("digest/personal-growth/speech.md"), b"# pg digest").expect("write");
            let index = json!({
                "entries": {
                    id_old: {
                        "common_path": "ai/old.md",
                        "created_at": "202606010001",
                        "layers": ["digest"]
                    },
                    id_new: {
                        "common_path": "ai/notes/new.md",
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
                    id_raw: {
                        "common_path": "ai/jot.md",
                        "created_at": "202606200000",
                        "layers": ["raw"]
                    }
                }
            });
            fs::write(notes.join("index.json"), index.to_string()).expect("write index");
        },
        move |cfg_dir| {
            let v = list_all_notes_catalogs(cfg_dir);
            let items = v["items"].as_array().expect("items");
            assert_eq!(items.len(), 2);
            let ai = items.iter().find(|i| i["catalog"] == "ai").expect("ai");
            assert_eq!(ai["note_id"], id_raw);
            assert_eq!(ai["created_at"], "202606200000");
            assert!(ai.get("content").is_none());
            let pg = items
                .iter()
                .find(|i| i["catalog"] == "personal-growth")
                .expect("pg");
            assert_eq!(pg["note_id"], id_pg);
            let listed = list_notes_by_catalog(cfg_dir, "ai");
            assert_eq!(
                listed["ids"],
                json!([id_raw, id_new, id_old])
            );
            let latest = get_latest_digest_per_catalog(cfg_dir);
            let latest_items = latest["items"].as_array().expect("latest items");
            let ai_digest = latest_items
                .iter()
                .find(|i| i["catalog"] == "ai")
                .expect("ai digest");
            assert_eq!(ai_digest["note_id"], id_new);
            assert_eq!(ai_digest["ok"], true);
            assert_eq!(ai_digest["content"], "# newest digest");
        },
    );
}

#[test]
fn get_note_digest_and_content_by_id() {
    let id_ok = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    with_notes_repo(
        move |_, notes| {
            fs::create_dir_all(notes.join("digest/ai")).expect("mkdir digest");
            fs::create_dir_all(notes.join("raw/ai")).expect("mkdir raw");
            fs::write(notes.join("digest/ai/note.md"), b"# digest body").expect("write digest");
            fs::write(notes.join("raw/ai/note.md"), b"# raw body").expect("write raw");
            let index = json!({
                "entries": {
                    id_ok: {
                        "common_path": "ai/note.md",
                        "created_at": "202606190004",
                        "layers": ["raw", "digest"]
                    }
                }
            });
            fs::write(notes.join("index.json"), index.to_string()).expect("write index");
        },
        move |cfg_dir| {
            let digest = get_note_digest_by_id(cfg_dir, id_ok);
            assert_eq!(digest["ok"], true);
            assert_eq!(digest["content"], "# digest body");
            let missing = get_note_digest_by_id(cfg_dir, "ffffffffffffffffffffffffffffffff");
            assert_eq!(missing["ok"], false);
            let bad = get_note_digest_by_id(cfg_dir, "bad");
            assert_eq!(bad["ok"], false);
            let raw = get_note_content_by_id(cfg_dir, id_ok);
            assert_eq!(raw["ok"], true);
            assert_eq!(raw["content"], "# raw body");
            assert_eq!(raw["truncated"], false);
            assert_eq!(get_note_digest_by_id(cfg_dir, "")["_status"], 400);
        },
    );
}

#[test]
fn get_note_content_truncates_over_10kb() {
    let id_ok = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let over = "你".repeat(4000);
    assert!(over.len() > RAW_CONTENT_MAX_BYTES);
    with_notes_repo(
        move |_, notes| {
            fs::create_dir_all(notes.join("raw/inbox")).expect("mkdir raw");
            fs::write(notes.join("raw/inbox/note.md"), over.as_bytes()).expect("write raw");
            let index = json!({
                "entries": {
                    id_ok: {
                        "common_path": "inbox/note.md",
                        "created_at": "202606190004",
                        "layers": ["raw"]
                    }
                }
            });
            fs::write(notes.join("index.json"), index.to_string()).expect("write index");
        },
        move |cfg_dir| {
            let raw = get_note_content_by_id(cfg_dir, id_ok);
            assert_eq!(raw["ok"], true);
            assert_eq!(raw["truncated"], true);
            let content = raw["content"].as_str().expect("content");
            assert!(content.len() <= RAW_CONTENT_MAX_BYTES);
        },
    );
}

#[test]
fn notes_search_filter_and_hit_projection() {
    assert_eq!(
        notes_raw_search_filter(None).expect("filter"),
        r#"layer = "raw""#
    );
    assert_eq!(
        notes_raw_search_filter(Some("inbox")).expect("filter"),
        r#"layer = "raw" AND common_path STARTS WITH "inbox/""#
    );
    assert_eq!(
        notes_raw_search_filter(Some("personal-growth")).expect("filter"),
        r#"layer = "raw" AND common_path STARTS WITH "personal-growth/""#
    );
    let bad = notes_raw_search_filter(Some(r#"inbox" OR layer = "digest"#));
    assert_eq!(bad.expect_err("reject injected catalog")["_status"], 400);

    let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut path_index = std::collections::HashMap::new();
    path_index.insert(
        "ai/note.md".to_string(),
        (id.to_string(), "202606190004".to_string()),
    );
    let hits = vec![
        json!({
            "layer": "digest",
            "common_path": "ai/note.md",
            "title": "digest title",
            "_formatted": { "body": "digest <em>hit</em>" }
        }),
        json!({
            "layer": "raw",
            "common_path": "ai/note.md",
            "title": "raw title",
            "_formatted": { "body": "raw <em>hit</em>" }
        }),
        json!({
            "layer": "raw",
            "common_path": "orphan.md",
            "title": "orphan",
            "body": "no index"
        }),
    ];
    let items = project_raw_search_hits(&hits, &path_index);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["note_id"], id);
    assert_eq!(items[0]["catalog"], "ai");
    assert_eq!(items[0]["title"], "raw title");
    assert_eq!(items[0]["created_at"], "202606190004");
    assert_eq!(items[0]["matches"][0]["snippet"], "raw <em>hit</em>");
    assert!(items[0].get("content").is_none());
}

#[test]
fn search_notes_rejects_empty_q() {
    with_notes_repo(
        |_, notes| {
            fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
        },
        |cfg_dir| {
            let v = search_notes(cfg_dir, "  ", None, None);
            assert_eq!(v["_status"], 400);
            assert_eq!(v["error"], "Missing q");
        },
    );
}

#[test]
fn get_annotation_decodes_percent_encoding() {
    with_notes_repo(
        |_, notes| {
            fs::create_dir_all(notes.join("annotations").join("ai")).expect("mkdir");
            let ann_path = notes.join("annotations/ai/note.json");
            fs::write(&ann_path, br#"{"done":true}"#).expect("write");
        },
        |cfg_dir| {
            let v = get_annotation(cfg_dir, "ai/note.md");
            assert_eq!(v["done"], json!(true));
        },
    );
}
