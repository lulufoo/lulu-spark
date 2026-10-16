use super::db::{open_with_steps, MIGRATIONS};
use super::*;

use rusqlite::Connection;
use serde_json::json;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).expect("count")
}

#[test]
fn v7_file_has_the_registry_and_empty_doc_tables_and_nothing_copied_yet() {
    let (_dir, path) = temp_db();
    let old = open_with_steps(&path, &MIGRATIONS[..6]).expect("v6");
    insert_note(&old, &NoteRow::new("n1", "a.md")).expect("note");
    drop(old);
    let v7 = open_with_steps(&path, &MIGRATIONS[..7]).expect("v7");
    assert_eq!(count(&v7, "workspace"), 2);
    assert_eq!(count(&v7, "doc_default"), 0);
    assert_eq!(count(&v7, "doc_knowledge"), 0);
    assert_eq!(count(&v7, "project"), 0);
    assert_eq!(count(&v7, "note"), 1);
}

#[test]
fn legacy_rows_are_copied_into_default_and_knowledge_and_the_old_tables_stay() {
    let (_dir, path) = temp_db();
    let old = open_with_steps(&path, &MIGRATIONS[..7]).expect("v7");
    insert_note_category(&old, &NoteCategoryRow::new("ops", "ops", "Ops", "ops desc")).expect("cat");
    insert_note(&old, &NoteRow::new("n1", "notes/a.md")).expect("n1");
    let mut n2 = NoteRow::new("n2", "notes/b.md");
    n2.category_id = "ops".into();
    n2.title = Some("B".into());
    n2.tags = json!(["x"]);
    n2.comments = json!([{ "id": "c1", "text": "t", "ts": "x" }]);
    insert_note(&old, &n2).expect("n2");

    for (id, category, description) in [
        ("demo", "AI", "about demo"),
        ("plain", UNCATEGORIZED, ""),
        ("inbox", "AI", ""),
    ] {
        insert_knowledge_directory(&old, &KnowledgeDirectoryRow::new(id, id, category, description))
            .expect("dir");
    }
    insert_knowledge_doc(&old, &KnowledgeDocRow::new("k1", "demo", "docs/a.md")).expect("k1");
    let mut gone = KnowledgeDocRow::new("k2", "demo", "docs/gone.md");
    gone.status = KnowledgeStatus::Missing;
    insert_knowledge_doc(&old, &gone).expect("k2");
    let mut k3 = KnowledgeDocRow::new("k3", "inbox", "x.md");
    k3.highlights = json!([{ "id": "h1", "text": "s", "occurrence": 1 }]);
    insert_knowledge_doc(&old, &k3).expect("k3");
    drop(old);

    let conn = open_at(&path).expect("→ latest");
    assert_eq!((count(&conn, "note"), count(&conn, "knowledge_doc")), (2, 3), "old tables untouched");

    let default_projects = list_projects(&conn, "default").expect("list");
    let ids: Vec<_> = default_projects.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["inbox", "ops"]);
    let ops = get_project(&conn, "ops").expect("get").expect("ops");
    assert_eq!((ops.folder.as_str(), ops.title.as_str(), ops.description.as_str()), ("ops", "Ops", "ops desc"));

    assert_eq!(list_projects(&conn, "knowledge").expect("list").len(), 3);
    let demo = get_project(&conn, "knowledge:demo").expect("get").expect("demo");
    assert_eq!((demo.workspace_id.as_str(), demo.folder.as_str()), ("knowledge", "demo"));
    assert_eq!(demo.description, "分组：AI\nabout demo");
    assert_eq!(get_project(&conn, "knowledge:plain").expect("get").expect("plain").description, "");
    assert_eq!(get_project(&conn, "knowledge:inbox").expect("get").expect("inbox").description, "分组：AI");
    assert_eq!(get_project(&conn, "inbox").expect("get").expect("notes inbox").workspace_id, "default");

    assert_eq!(count(&conn, "doc_default"), 2);
    let (raw, project, title, tags, comments): (String, String, Option<String>, String, String) = conn
        .query_row(
            "SELECT raw_path, project_id, title, tags, comments FROM doc_default WHERE doc_id = 'n2'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .expect("n2 row");
    assert_eq!((raw.as_str(), project.as_str(), title.as_deref()), ("notes/b.md", "ops", Some("B")));
    assert_eq!(tags, r#"["x"]"#);
    assert!(comments.contains("c1"));

    assert_eq!(count(&conn, "doc_knowledge"), 2, "the missing record is not copied");
    let copied: Vec<(String, String, String)> = conn
        .prepare("SELECT doc_id, raw_path, project_id FROM doc_knowledge ORDER BY doc_id")
        .expect("prepare")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(
        copied,
        vec![
            ("k1".to_string(), "docs/a.md".to_string(), "knowledge:demo".to_string()),
            ("k3".to_string(), "x.md".to_string(), "knowledge:inbox".to_string())
        ]
    );
    let hl: String = conn
        .query_row("SELECT highlights FROM doc_knowledge WHERE doc_id = 'k3'", [], |r| r.get(0))
        .expect("k3");
    assert!(hl.contains("h1"));
}
