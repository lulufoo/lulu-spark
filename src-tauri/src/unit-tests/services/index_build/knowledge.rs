use std::collections::HashSet;
use std::fs;

use super::*;
use crate::services::sediment_kb;
use crate::services::spark_data::{self, find_knowledge_doc_id, get_knowledge_directory_by_folder};
use crate::test_support::TestSandbox;

fn write(sandbox: &TestSandbox, rel: &str) -> std::path::PathBuf {
    let file = sandbox.knowledge_root().join("demo").join(rel);
    fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    fs::write(&file, format!("# {rel}\n\nsome text for {rel}")).expect("write");
    file
}

fn indexed_doc_ids(sandbox: &TestSandbox) -> HashSet<String> {
    search_index::collect_knowledge(sandbox.config_dir(), None)
        .expect("collect")
        .into_iter()
        .map(|c| c.doc_id)
        .collect()
}

#[test]
fn indexed_doc_ids_are_the_registry_ids_of_active_files_only() {
    let sandbox = TestSandbox::new();
    sediment_kb::ensure_uncategorized().expect("ensure");
    sediment_kb::add_directory("demo").expect("add");
    write(&sandbox, "a.md");
    let gone = write(&sandbox, "b.md");
    scan_registered_repos(sandbox.config_dir(), None).expect("scan");
    let conn = spark_data::open().expect("db");
    let demo = get_knowledge_directory_by_folder(&conn, "demo")
        .expect("dir")
        .expect("demo")
        .id;
    let a = find_knowledge_doc_id(&conn, &demo, "a.md").expect("find").expect("a");
    let b = find_knowledge_doc_id(&conn, &demo, "b.md").expect("find").expect("b");
    assert_eq!(indexed_doc_ids(&sandbox), HashSet::from([a.clone(), b.clone()]));

    fs::remove_file(&gone).expect("remove");
    scan_registered_repos(sandbox.config_dir(), None).expect("rescan");
    assert_eq!(indexed_doc_ids(&sandbox), HashSet::from([a]), "missing docs leave the index");
}

#[test]
fn a_file_the_registry_has_not_seen_is_not_indexed() {
    let sandbox = TestSandbox::new();
    sediment_kb::ensure_uncategorized().expect("ensure");
    sediment_kb::add_directory("demo").expect("add");
    write(&sandbox, "fresh.md");
    assert!(indexed_doc_ids(&sandbox).is_empty());
}
