use super::*;
use std::fs;

use crate::test_support::TestSandbox;

#[test]
fn save_entry_updates_file() {
    let sandbox = TestSandbox::new();
    let notes = sandbox.spark_root().join("notes");
    let md = notes.join("digest").join("foo.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "old").expect("w");

    let v = save_entry(
        sandbox.config_dir(),
        "digest".into(),
        "foo.md".into(),
        "new content".into(),
    );
    assert_eq!(v["ok"], json!(true));
    assert_eq!(fs::read_to_string(&md).expect("read"), "new content");
}

/// W6-5: App-internal save refreshes the keyword index for that one file, and a
/// vanished file drops its rows — without a full rebuild.
#[test]
fn save_entry_syncs_keyword_index_for_touched_file() {
    use crate::services::keyword_index::{open, search, search_desktop_workbench, SearchFilter};

    let sandbox = TestSandbox::new();
    let notes = sandbox.spark_root().join("notes");
    let md = notes.join("raw").join("proj").join("sync.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# Sync\n\nold body without keywords").expect("w");
    // An index file must exist; the hook is a no-op otherwise.
    drop(open(&sandbox.cache_dir()).expect("create index"));

    let v = save_entry(
        sandbox.config_dir(),
        "raw".into(),
        "proj/sync.md".into(),
        "# Sync\n\n这里新增角色扮演关键词".into(),
    );
    assert_eq!(v["ok"], json!(true));
    let out = search_desktop_workbench(&sandbox.cache_dir(), "角色扮演", Some(5));
    let hits = out["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1, "{out}");
    assert_eq!(hits[0]["common_path"], "proj/sync.md");

    fs::remove_file(&md).expect("rm");
    crate::services::keyword_index::sync_note_files(sandbox.config_dir(), &[("raw", "proj/sync.md")])
        .expect("sync delete");
    let conn = open(&sandbox.cache_dir()).expect("open");
    let filter = SearchFilter {
        category: Some("notes".into()),
        ..SearchFilter::default()
    };
    assert!(search(&conn, "角色扮演", &filter, 5).expect("search").is_empty());
}
