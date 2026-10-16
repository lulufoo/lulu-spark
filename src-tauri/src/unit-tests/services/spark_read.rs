use super::*;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::test_support::{seed_note_entries, TestSandbox};

#[test]
fn get_config_has_frontend_contract_keys() {
    let _sandbox = TestSandbox::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let v = get_config(&root);
    assert!(v.get("spark_root").is_some());
    assert!(v.get("knowledge_root").is_some());
    assert!(v.get("notes_root").is_some());
    assert!(v.get("github_user_url").is_none());
    assert!(v.get("spark_github_repo_url").is_none());
    assert!(v.get("assistant_engine").is_some());
    assert!(v.get("theme").is_some());
    assert!(v.get("cache_dir").is_some());
    assert!(v.get("has_github_token").is_none());
    assert!(v.get("has_host_key").is_none());
    assert!(v.get("has_cursor_key").is_none());
    assert!(v.get("llm").is_some());
    assert!(v.get("api_key").is_none());
    assert!(v.get("api_key_host").is_none());
    assert!(v.get("api_key_cursor").is_none());
    assert!(v["llm"].get("api_key").is_none());
    assert!(v["llm"]["host"].get("key_saved").is_some());
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
    let notes = sandbox.data_dir().join("notes");
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
    use crate::services::sediment_kb::{add_directory, ensure_uncategorized, UNCATEGORIZED_ID};

    with_sediment_kb_topics_cache(|_cfg, wb| {
        ensure_uncategorized().expect("ensure");
        add_directory("kb-a").expect("add");

        let v = get_topics(wb);
        assert!(v.get("error").is_none(), "unexpected error: {v:?}");
        assert_eq!(v["source"], "sediment-kb");
        let topics = v["topics"].as_array().expect("topics array");
        let kb = topics
            .iter()
            .find(|t| t.get("repo") == Some(&serde_json::json!("kb-a")))
            .expect("sediment-kb repo topic");
        assert_eq!(kb["description"], "");
        assert_eq!(kb["category_id"], UNCATEGORIZED_ID);
        assert_eq!(kb["category_name"], "Uncategorized");
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
        add_directory, ensure_uncategorized, list_repos_for_topics, UNCATEGORIZED_ID,
    };

    with_sediment_kb_topics_cache(|_, _| {
        ensure_uncategorized().expect("ensure");
        add_directory("demo").expect("add");

        let rows = list_repos_for_topics().expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].repo, "demo");
        assert_eq!(rows[0].category_id, UNCATEGORIZED_ID);
        assert_eq!(rows[0].category_name, "Uncategorized");
    });
}

#[test]
fn get_draft_invalid_id() {
    let dir = tempfile::tempdir().expect("tmp");
    let v = get_draft(dir.path(), "..%2Fsecret");
    assert_eq!(v["error"], "Invalid id");
}

#[test]
fn check_spark_root_requires_an_existing_directory() {
    let dir = tempfile::tempdir().expect("tmp");
    assert_eq!(check_spark_root("/no/such/spark")["ok"], false);
    assert_eq!(check_spark_root("  ")["ok"], false);
    assert_eq!(check_spark_root(dir.path().to_str().unwrap())["ok"], true);
}

#[test]
fn get_notes_index_is_built_from_the_note_table_in_the_old_entry_shape() {
    with_notes_repo(
        |_, _| {
            seed_note_entries(&json!({
                "n1": {
                    "common_path": "ai/topic/20260101-a.md", "created_at": "202601010000",
                    "layers": ["raw", "digest"], "source_type": "article", "title": "A",
                    "translations": { "zh": "ai/topic/20260101-a-zh.md" }
                },
                "n2": { "common_path": "ai/topic/20260102-b.md", "layers": ["raw"] },
            }));
        },
        |cfg_dir| {
            let v = get_notes_index(cfg_dir);
            assert_eq!(
                v,
                json!({ "entries": {
                    "n1": {
                        "id": "n1", "category_id": "ai", "folder": "ai",
                        "created_at": "202601010000",
                        "layers": ["raw", "digest"], "source_type": "article", "title": "A",
                        "translations": { "zh": "topic/20260101-a-zh.md" }
                    },
                    "n2": {
                        "id": "n2", "category_id": "ai", "folder": "ai",
                        "layers": ["raw"]
                    },
                }})
            );
        },
    );
}

#[test]
fn get_notes_index_of_an_empty_table_has_no_entries() {
    with_notes_repo(
        |_, _| {},
        |cfg_dir| assert_eq!(get_notes_index(cfg_dir), json!({ "entries": {} })),
    );
}

#[test]
fn get_notes_file_rejects_traversal() {
    with_notes_repo(
        |_, notes| {
            fs::create_dir_all(notes.join("raw")).expect("mkdir");
        },
        |cfg_dir| {
            let v = get_notes_file(cfg_dir, "raw", "../index.json", None);
            assert_eq!(v["error"], "Invalid id");
        },
    );
}

#[test]
fn get_annotations_summary_includes_resolved_tags() {
    use crate::services::spark_data::{get_note, open, update_note};
    use crate::services::tag_write::tag_attach;
    use crate::test_support::with_sandbox_notes;

    with_sandbox_notes(|dir, _| {
        let cp = "ai/tags.md";
        let id = format!("{:0>32}", "e1");
        seed_note_entries(&serde_json::json!({ id.clone(): { "common_path": cp } }));
        let known = tag_attach(dir, &id, &serde_json::json!({ "value": "Known" }));
        let known = known["key"].as_str().expect("key").to_string();
        // A row can reference a key the tag table does not have.
        let conn = open().expect("db");
        let mut note = get_note(&conn, &id).expect("get").expect("row");
        note.tags = serde_json::json!([known, "ghostkey123456"]);
        update_note(&conn, &note).expect("update");

        let summary = get_annotations_summary(dir);
        let entry = summary[&id].as_object().expect("entry");
        let tags = entry["tags"].as_array().expect("tags");
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().any(|t| t["key"] == known && t["value"] == "Known"));
        assert!(
            tags.iter()
                .any(|t| t["key"] == "ghostkey123456" && t["unknown"] == true)
        );
    });
}

#[test]
fn get_notes_file_returns_content() {
    let id = "cccccccccccccccccccccccccccccccc";
    with_notes_repo(
        move |_, notes| {
            let raw = notes.join("raw").join("inbox").join("a.md");
            fs::create_dir_all(raw.parent().unwrap()).expect("mkdir");
            fs::write(&raw, b"hello").expect("write");
            seed_note_entries(&json!({ id: { "common_path": "a.md" } }));
        },
        move |cfg_dir| {
            let v = get_notes_file(cfg_dir, "raw", id, None);
            assert_eq!(v["content"], "hello");
        },
    );
}

#[test]
fn get_notes_asset_returns_png_bytes() {
    let id = "dddddddddddddddddddddddddddddddd";
    with_notes_repo(
        move |_, notes| {
            let png = notes.join("raw").join("ai").join("note.png");
            fs::create_dir_all(png.parent().unwrap()).expect("mkdir");
            fs::write(&png, b"\x89PNG\r\n").expect("write");
            fs::write(notes.join("raw").join("ai").join("note.md"), b"# x").expect("md");
            seed_note_entries(&json!({ id: { "common_path": "ai/note.md" } }));
        },
        move |cfg_dir| {
            let v = get_notes_asset(cfg_dir, "raw", id, "note.png", None);
            assert!(v.get("data_b64").and_then(|x| x.as_str()).is_some());
            assert_eq!(v["mime_type"], "image/png");
        },
    );
}

#[test]
fn get_notes_asset_rejects_traversal() {
    let id = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    with_notes_repo(
        move |_, notes| {
            fs::create_dir_all(notes.join("raw").join("ai")).expect("mkdir");
            fs::write(notes.join("raw").join("ai").join("note.md"), b"# x").expect("md");
            seed_note_entries(&json!({ id: { "common_path": "ai/note.md" } }));
        },
        move |cfg_dir| {
            let v = get_notes_asset(cfg_dir, "raw", id, "../index.json", None);
            assert_eq!(v["error"], "Invalid path");
        },
    );
}

#[test]
fn get_notes_asset_rejects_non_whitelist_ext() {
    let id = "ffffffffffffffffffffffffffffffff";
    with_notes_repo(
        move |_, notes| {
            let svg = notes.join("raw").join("inbox").join("x.svg");
            fs::create_dir_all(svg.parent().unwrap()).expect("mkdir");
            fs::write(&svg, b"<svg").expect("write");
            fs::write(notes.join("raw").join("inbox").join("x.md"), b"#").expect("md");
            seed_note_entries(&json!({ id: { "common_path": "x.md" } }));
        },
        move |cfg_dir| {
            let v = get_notes_asset(cfg_dir, "raw", id, "x.svg", None);
            assert_eq!(v["error"], "Unsupported media type");
        },
    );
}

#[test]
fn catalog_listing_helpers_pick_newest_including_raw_only() {
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
            seed_note_entries(&index["entries"]);
        },
        move |cfg_dir| {
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
            seed_note_entries(&index["entries"]);
        },
        move |cfg_dir| {
            let digest = get_note_digest(cfg_dir, id_ok);
            assert_eq!(digest["ok"], true);
            assert_eq!(digest["content"], "# digest body");
            let missing = get_note_digest(cfg_dir, "ffffffffffffffffffffffffffffffff");
            assert_eq!(missing["ok"], false);
            let bad = get_note_digest(cfg_dir, "bad");
            assert_eq!(bad["ok"], false);
            let raw = get_note_content_by_id(cfg_dir, id_ok);
            assert_eq!(raw["ok"], true);
            assert_eq!(raw["content"], "# raw body");
            assert_eq!(raw["truncated"], false);
            assert_eq!(get_note_digest(cfg_dir, "")["_status"], 400);
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
            seed_note_entries(&index["entries"]);
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
fn notes_search_hit_projection() {
    let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut entries = serde_json::Map::new();
    entries.insert(
        id.to_string(),
        json!({ "folder": "ai", "created_at": "202606190004" }),
    );
    let hits = vec![
        json!({
            "layer": "digest",
            "note_id": id,
            "title": "digest title",
            "_formatted": { "body": "digest <em>hit</em>" }
        }),
        json!({
            "layer": "raw",
            "note_id": id,
            "title": "raw title",
            "_formatted": { "body": "raw <em>hit</em>" }
        }),
        json!({
            "layer": "raw",
            "note_id": "orphanorphanorphanorphanorphanx",
            "title": "orphan",
            "body": "no index"
        }),
    ];
    let items = project_raw_search_hits(&hits, &entries);
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
        |_, _| {},
        |cfg_dir| {
            let v = search_notes(cfg_dir, "  ", None);
            assert_eq!(v["_status"], 400);
            assert_eq!(v["error"], "Missing q");
        },
    );
}

#[test]
fn get_annotation_decodes_percent_encoding() {
    with_notes_repo(
        |_, _| {
            let id = "a".repeat(32);
            seed_note_entries(&json!({ id.clone(): { "common_path": "ai/note.md" } }));
            let dir = std::path::Path::new("");
            assert_eq!(crate::services::annotation::set_done(dir, &id, true)["ok"], json!(true));
        },
        |cfg_dir| {
            let id = "a".repeat(32);
            let v = get_annotation(cfg_dir, &id);
            assert_eq!(v["done"], json!(true));
        },
    );
}
