use super::*;
use crate::services::spark_data::{self, get_knowledge_doc, KnowledgeStatus};
use crate::test_support::TestSandbox;
use serde_json::json;
use std::fs;

const REPO: &str = "myrepo";

fn no_hide() -> Vec<Regex> {
    Vec::new()
}

fn hide(patterns: &[&str]) -> Vec<Regex> {
    patterns.iter().map(|p| Regex::new(p).expect("regex")).collect()
}

fn write(kb: &Path, rel: &str) {
    let path = kb.join(REPO).join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, "text").expect("write");
}

fn status_of(conn: &rusqlite::Connection, rel: &str) -> Option<KnowledgeStatus> {
    let id = spark_data::find_knowledge_doc_id(conn, REPO, rel).expect("find")?;
    Some(get_knowledge_doc(conn, &id).expect("get").expect("row").status)
}

#[test]
fn scan_registers_new_markdown_files_once() {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    write(&kb, "a.md");
    write(&kb, "docs/b.md");
    write(&kb, "docs/notes.txt");
    let conn = spark_data::open().expect("db");

    let first = scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan");
    assert_eq!(first.registered, 2);
    let id = spark_data::find_knowledge_doc_id(&conn, REPO, "docs/b.md").expect("find");
    assert!(id.is_some());
    assert!(status_of(&conn, "docs/notes.txt").is_none(), "only .md files are registered");

    let second = scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("rescan");
    assert_eq!(second, ReconcileStats::default());
    assert_eq!(
        spark_data::find_knowledge_doc_id(&conn, REPO, "docs/b.md").expect("find"),
        id,
        "ids are stable across scans"
    );
}

#[test]
fn vanished_file_goes_missing_keeps_comments_and_comes_back() {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    write(&kb, "a.md");
    let conn = spark_data::open().expect("db");
    scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan");
    let id = spark_data::find_knowledge_doc_id(&conn, REPO, "a.md").expect("find").expect("id");
    let mut row = get_knowledge_doc(&conn, &id).expect("get").expect("row");
    row.comments = json!([{ "id": "c1", "text": "still here", "ts": "t" }]);
    spark_data::update_knowledge_doc(&conn, &row).expect("comment");

    fs::remove_file(kb.join(REPO).join("a.md")).expect("outside delete");
    let gone = scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan after delete");
    assert_eq!(gone.marked_missing, 1);
    assert_eq!(status_of(&conn, "a.md"), Some(KnowledgeStatus::Missing));
    let kept = get_knowledge_doc(&conn, &id).expect("get").expect("record kept");
    assert_eq!(kept.comments[0]["text"], "still here");

    write(&kb, "a.md");
    let back = scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan after restore");
    assert_eq!(back.restored, 1);
    let restored = get_knowledge_doc(&conn, &id).expect("get").expect("same record");
    assert_eq!(restored.status, KnowledgeStatus::Active);
    assert_eq!(restored.comments[0]["text"], "still here", "same id, comments intact");
}

#[test]
fn missing_clone_directory_is_not_a_mass_deletion() {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    write(&kb, "a.md");
    write(&kb, "b.md");
    let conn = spark_data::open().expect("db");
    scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan");

    fs::rename(kb.join(REPO), kb.join("elsewhere")).expect("clone temporarily gone");
    let stats = scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan without the clone");
    assert_eq!(stats, ReconcileStats::default());
    assert_eq!(status_of(&conn, "a.md"), Some(KnowledgeStatus::Active));
    assert_eq!(status_of(&conn, "b.md"), Some(KnowledgeStatus::Active));
}

#[test]
fn hidden_paths_are_not_registered_and_never_marked_missing() {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    write(&kb, "a.md");
    let conn = spark_data::open().expect("db");
    scan_repo_dir(&conn, &kb, REPO, &no_hide()).expect("scan");
    // A record under what is now a hidden folder; its file is not on disk.
    spark_data::register_knowledge_doc(&conn, REPO, "secret/old.md").expect("record");
    write(&kb, "secret/new.md");

    let stats = scan_repo_dir(&conn, &kb, REPO, &hide(&["^secret$"])).expect("scan with hide rule");
    assert_eq!(stats.marked_missing, 0, "hidden is not deleted");
    assert_eq!(status_of(&conn, "secret/old.md"), Some(KnowledgeStatus::Active));
    assert!(status_of(&conn, "secret/new.md").is_none(), "hidden files are not registered");
    assert_eq!(status_of(&conn, "a.md"), Some(KnowledgeStatus::Active));
}

#[test]
fn scan_registered_repos_reads_the_repo_list_and_honours_force_repo() {
    let sandbox = TestSandbox::new();
    crate::services::sediment_kb::add_directory("myrepo").expect("myrepo");
    crate::services::sediment_kb::add_directory("other").expect("other");
    let kb = sandbox.knowledge_root();
    write(&kb, "a.md");
    fs::write(kb.join("other/z.md"), "z").expect("z");

    scan_registered_repos(sandbox.config_dir(), Some("myrepo")).expect("forced scan");
    let conn = spark_data::open().expect("db");
    assert!(spark_data::find_knowledge_doc_id(&conn, "myrepo", "a.md").expect("find").is_some());
    assert!(spark_data::find_knowledge_doc_id(&conn, "other", "z.md").expect("find").is_none());

    scan_registered_repos(sandbox.config_dir(), None).expect("full scan");
    assert!(spark_data::find_knowledge_doc_id(&conn, "other", "z.md").expect("find").is_some());
}
