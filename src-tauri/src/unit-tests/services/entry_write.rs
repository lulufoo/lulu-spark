use super::*;
use std::fs;

use crate::test_support::{seed_note_entries, TestSandbox};

#[test]
fn save_entry_updates_file() {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    let md = notes.join("digest").join(crate::test_support::default_inbox_id()).join("foo.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "old").expect("w");
    let id = "a".repeat(32);
    seed_note_entries(&json!({ id.clone(): { "common_path": "foo.md", "layers": ["raw", "digest"] } }));

    let v = save_entry(
        sandbox.config_dir(),
        "digest".into(),
        id,
        "new content".into(),
        None,
    );
    assert_eq!(v["ok"], json!(true));
    assert_eq!(fs::read_to_string(&md).expect("read"), "new content");
}

/// W6-5: App-internal save refreshes the search index for that one file, and a
/// vanished file drops its rows — without a full rebuild.
#[test]
fn save_entry_syncs_search_index_for_touched_file() {
    use crate::services::search_index::{open, search, search_desktop_docs, SearchFilter};

    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    let md = notes.join("raw").join("seed-proj").join("sync.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# Sync\n\nold body without keywords").expect("w");
    let id = "b".repeat(32);
    seed_note_entries(&json!({ id.clone(): { "common_path": "proj/sync.md" } }));
    drop(open(&sandbox.cache_dir()).expect("create index"));

    let v = save_entry(
        sandbox.config_dir(),
        "raw".into(),
        id.clone(),
        "# Sync\n\n这里新增角色扮演关键词".into(),
        None,
    );
    assert_eq!(v["ok"], json!(true));
    let out = search_desktop_docs(&sandbox.cache_dir(), "角色扮演", Some(5), None);
    let hits = out["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1, "{out}");
    assert_eq!(hits[0]["id"], id);
    assert_eq!(hits[0]["project_id"], "seed-proj");
    assert!(hits[0].get("common_path").is_none());

    fs::remove_file(&md).expect("rm");
    crate::services::search_index::sync_workspace_docs(&crate::test_support::default_workspace().id, &[&id])
        .expect("sync delete");
    let conn = open(&sandbox.cache_dir()).expect("open");
    let filter = SearchFilter {
        category: Some("docs".into()),
        ..SearchFilter::default()
    };
    assert!(search(&conn, "角色扮演", &filter, 5).expect("search").is_empty());
}

#[test]
fn save_entry_keeps_orphan_attachment_files() {
    use crate::services::docs::create_doc;

    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage");
    let md = dir.join("source.md");
    fs::write(&md, "# 标题\n\n正文足够长。\n\n![a](keep.png)\n![b](gone.png)\n").expect("md");
    let keep = dir.join("keep.png");
    let gone = dir.join("gone.png");
    fs::write(&keep, b"\x89PNG\r\n").expect("keep");
    fs::write(&gone, b"\x89PNG\r\n").expect("gone");
    let created = create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": md.canonicalize().unwrap().to_str().unwrap(),
            "title": "标题",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [
                keep.canonicalize().unwrap().to_str().unwrap(),
                gone.canonicalize().unwrap().to_str().unwrap()
            ],
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap().to_string();
    let attached = notes
        .join("raw")
        .join(crate::test_support::default_inbox_id())
        .join("notes")
        .join(".attached")
        .join(&id);
    let v = save_entry(
        sandbox.config_dir(),
        "raw".into(),
        id.clone(),
        format!("# 标题\n\n正文足够长。\n\n![a](.attached/{id}/keep.png)\n"),
        None,
    );
    assert_eq!(v["ok"], json!(true), "{v}");
    assert!(attached.join("keep.png").is_file());
    assert!(attached.join("gone.png").is_file());
}
