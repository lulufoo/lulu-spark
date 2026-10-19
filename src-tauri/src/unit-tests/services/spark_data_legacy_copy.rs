use super::db::{open_with_steps, MIGRATIONS};
use super::*;

use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).expect("count")
}

fn tables(conn: &Connection) -> Vec<String> {
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .expect("prepare")
        .query_map([], |r| r.get::<_, String>(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

#[test]
fn v7_file_has_the_registry_and_empty_doc_tables_and_nothing_copied_yet() {
    let (_dir, path) = temp_db();
    let old = open_with_steps(&path, &MIGRATIONS[..6]).expect("v6");
    old.execute("INSERT INTO note (doc_id, raw_path) VALUES ('n1', 'a.md')", []).expect("note");
    drop(old);
    let v7 = open_with_steps(&path, &MIGRATIONS[..7]).expect("v7");
    assert_eq!(count(&v7, "workspace"), 2);
    assert_eq!(count(&v7, "doc_default"), 0);
    assert_eq!(count(&v7, "doc_knowledge"), 0);
    assert_eq!(count(&v7, "project"), 0);
    assert_eq!(count(&v7, "note"), 1);
}

#[test]
fn legacy_rows_are_copied_into_default_and_knowledge_and_the_old_tables_are_dropped() {
    let (_dir, path) = temp_db();
    let old = open_with_steps(&path, &MIGRATIONS[..7]).expect("v7");
    old.execute("INSERT INTO note_category (id, folder, title, description) VALUES ('ops', 'ops', 'Ops', 'ops desc')", [])
        .expect("cat");
    old.execute("INSERT INTO note (doc_id, raw_path) VALUES ('n1', 'notes/a.md')", []).expect("n1");
    old.execute(
        "INSERT INTO note (doc_id, raw_path, title, tags, comments, category_id)
         VALUES ('n2', 'notes/b.md', 'B', '[\"x\"]', '[{\"id\":\"c1\"}]', 'ops')",
        [],
    )
    .expect("n2");
    for (id, category, description) in [
        ("demo", "AI", "about demo"),
        ("plain", "Uncategorized", ""),
        ("inbox", "AI", ""),
    ] {
        old.execute(
            "INSERT INTO knowledge_directory (id, folder, category, description) VALUES (?1, ?1, ?2, ?3)",
            [id, category, description],
        )
        .expect("dir");
    }
    old.execute("INSERT INTO knowledge_doc (doc_id, path, directory_id) VALUES ('k1', 'docs/a.md', 'demo')", [])
        .expect("k1");
    old.execute(
        "INSERT INTO knowledge_doc (doc_id, path, directory_id, status) VALUES ('k2', 'docs/gone.md', 'demo', 'missing')",
        [],
    )
    .expect("k2");
    old.execute(
        "INSERT INTO knowledge_doc (doc_id, path, directory_id, highlights)
         VALUES ('k3', 'x.md', 'inbox', '[{\"id\":\"h1\"}]')",
        [],
    )
    .expect("k3");
    drop(old);

    let conn = open_at(&path).expect("→ latest");
    let default = get_default_workspace(&conn).expect("default");
    let knowledge = get_workspace_by_name(&conn, "Knowledge").expect("lookup").expect("Knowledge");
    let mut names = tables(&conn);
    names.sort();
    let mut expected = vec![default.doc_table.clone(), knowledge.doc_table.clone(), "project".into(), "tag".into(), "workspace".into()];
    expected.sort();
    assert_eq!(names, expected);

    let default_projects = list_projects(&conn, &default.id).expect("list");
    let ids: Vec<_> = default_projects.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["inbox", "ops"]);
    let ops = get_project(&conn, "ops").expect("get").expect("ops");
    assert_eq!((ops.name.as_str(), ops.description.as_str()), ("Ops", "ops desc"));

    assert_eq!(list_projects(&conn, &knowledge.id).expect("list").len(), 3);
    let demo = get_project(&conn, "knowledge:demo").expect("get").expect("demo");
    assert_eq!(demo.workspace_id.as_str(), knowledge.id.as_str());
    assert_eq!(demo.description, "分组：AI\nabout demo");
    assert_eq!(get_project(&conn, "knowledge:plain").expect("get").expect("plain").description, "");
    assert_eq!(get_project(&conn, "knowledge:inbox").expect("get").expect("inbox").description, "分组：AI");
    assert_eq!(get_project(&conn, "inbox").expect("get").expect("notes inbox").workspace_id, default.id);

    assert_eq!(count(&conn, &default.doc_table), 2);
    let n2 = get_doc(&conn, &default.doc_table, "n2").expect("get").expect("n2");
    assert_eq!((n2.raw_path.as_str(), n2.project_id.as_str(), n2.title.as_deref()), ("notes/b.md", "ops", Some("B")));
    assert_eq!(n2.tags, serde_json::json!(["x"]));
    assert!(n2.comments.to_string().contains("c1"));

    assert_eq!(count(&conn, &knowledge.doc_table), 2, "the missing record is not copied");
    let k1 = get_doc(&conn, &knowledge.doc_table, "k1").expect("get").expect("k1");
    assert_eq!((k1.raw_path.as_str(), k1.project_id.as_str()), ("docs/a.md", "knowledge:demo"));
    let k3 = get_doc(&conn, &knowledge.doc_table, "k3").expect("get").expect("k3");
    assert_eq!((k3.raw_path.as_str(), k3.project_id.as_str()), ("x.md", "knowledge:inbox"));
    assert!(k3.highlights.to_string().contains("h1"));
    assert!(get_doc(&conn, &knowledge.doc_table, "k2").expect("get").is_none());
}
