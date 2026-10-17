use super::*;

use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")
        .expect("prepare")
        .exists([name])
        .expect("exists")
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).expect("count")
}

/// (name, type, notnull, default) per column, in table order.
fn columns(conn: &Connection, table: &str) -> Vec<(String, String, i64, Option<String>)> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .expect("pragma")
        .query_map([], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .expect("cols")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn insert_doc(conn: &Connection, table: &str, doc_id: &str, project_id: &str) -> rusqlite::Result<usize> {
    conn.execute(
        &format!("INSERT INTO {table}(doc_id, raw_path, project_id) VALUES (?1, 'a.md', ?2)"),
        [doc_id, project_id],
    )
}

#[test]
fn seed_registry_rows_and_modes_are_right() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let rows = list_workspaces(&conn).expect("list");
    let seen: Vec<_> = rows.iter().map(|w| (w.id.as_str(), w.name.as_str(), w.view_mode, w.is_default)).collect();
    assert_eq!(
        seen,
        vec![
            ("default", "Default", ViewMode::Directory, true),
            ("knowledge", "Knowledge", ViewMode::Timeline, false)
        ]
    );
    assert_eq!(rows[0].doc_table, "doc_default");
    assert_eq!(rows[1].doc_table, "doc_knowledge");
}

#[test]
fn migration_and_runtime_templates_make_the_same_doc_table() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Scratch", None).expect("create");
    assert_eq!(columns(&conn, "doc_default"), columns(&conn, &made.doc_table));
    assert_eq!(columns(&conn, "doc_knowledge"), columns(&conn, &made.doc_table));
    let names: Vec<String> = columns(&conn, &made.doc_table).into_iter().map(|c| c.0).collect();
    assert!(names.contains(&"project_id".to_string()));
    assert!(!names.contains(&"status".to_string()));
    assert!(!names.contains(&"category_id".to_string()));
}

#[test]
fn create_workspace_registers_row_and_table_in_one_step() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "  Ai Memory  ", None).expect("create");
    assert_eq!(made.name, "Ai Memory");
    assert_eq!(made.view_mode, ViewMode::Directory);
    assert!(!made.is_default);
    assert!(validate_workspace_id(&made.id).is_ok(), "id {:?}", made.id);
    assert_eq!(made.doc_table, format!("doc_{}", made.id));
    assert!(table_exists(&conn, &made.doc_table));
    assert_eq!(get_workspace(&conn, &made.id).expect("get"), Some(made.clone()));

    let timeline = create_workspace(&conn, "Log", Some(ViewMode::Timeline)).expect("create");
    assert_eq!(timeline.view_mode, ViewMode::Timeline);
    assert_ne!(timeline.id, made.id);
}

#[test]
fn create_workspace_rejects_blank_and_duplicate_names_without_leaving_a_table() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let tables = count(&conn, "sqlite_master");
    assert!(create_workspace(&conn, "   ", None).is_err());
    assert!(create_workspace(&conn, "Default", None).is_err(), "name is unique");
    assert_eq!(count(&conn, "sqlite_master"), tables);
    assert_eq!(count(&conn, "workspace"), 2);
}

#[test]
fn table_names_with_anything_but_doc_prefix_and_lowercase_id_are_refused() {
    for ok in ["doc_default", "doc_a1_b2", "doc_0"] {
        assert!(validate_doc_table_name(ok).is_ok(), "{ok}");
    }
    for bad in [
        "", "doc_", "note", "docs_a", "DOC_a", "doc_A", "doc_a-b", "doc_a b", "doc_a;DROP TABLE note",
        "doc_a\"", "doc_ä", "doc_a.b", format!("doc_{}", "x".repeat(60)).as_str(),
    ] {
        assert!(validate_doc_table_name(bad).is_err(), "{bad:?}");
    }
    assert_eq!(doc_table_for_id("ws_1").expect("ok"), "doc_ws_1");
    assert!(doc_table_for_id("Ws").is_err());
    assert!(doc_table_for_id("a;b").is_err());
}

#[test]
fn default_workspace_cannot_be_disabled_and_not_deleted_even_by_raw_sql() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    assert!(disable_workspace(&conn, "default").is_err());
    assert!(discard_new_workspace(&conn, "default").is_err());
    assert!(conn.execute("DELETE FROM workspace WHERE id = 'default'", []).is_err(), "trigger");
    assert!(get_workspace(&conn, "default").expect("get").is_some());
    assert!(table_exists(&conn, "doc_default"));
}

#[test]
fn only_one_workspace_can_be_default() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Other", None).expect("create");
    assert!(conn
        .execute("UPDATE workspace SET is_default = 1 WHERE id = ?1", [&made.id])
        .is_err());
}

#[test]
fn discard_drops_a_fresh_workspace_without_documents_and_refuses_one_with_documents() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Temp", None).expect("create");
    insert_project(&conn, &ProjectRow::new("p1", &made.id, "Notes", "")).expect("project");
    insert_doc(&conn, &made.doc_table, "d1", "p1").expect("doc");
    assert!(discard_new_workspace(&conn, &made.id).is_err(), "has a document");
    assert!(table_exists(&conn, &made.doc_table));
    assert!(get_project(&conn, "p1").expect("get").is_some());

    conn.execute(&format!("DELETE FROM {} WHERE doc_id = 'd1'", made.doc_table), []).expect("remove doc");
    assert!(discard_new_workspace(&conn, &made.id).expect("discard"), "the empty project goes with it");
    assert!(get_project(&conn, "p1").expect("get").is_none());
    assert!(!table_exists(&conn, &made.doc_table));
    assert!(get_workspace(&conn, &made.id).expect("get").is_none());
    assert!(!discard_new_workspace(&conn, &made.id).expect("again"), "already gone");
}

#[test]
fn disable_hides_a_workspace_keeps_its_data_and_enable_brings_it_back() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Temp", None).expect("create");
    insert_project(&conn, &ProjectRow::new("p1", &made.id, "Notes", "")).expect("project");
    insert_doc(&conn, &made.doc_table, "d1", "p1").expect("doc");
    assert!(made.disabled_at.is_none());

    assert!(disable_workspace(&conn, &made.id).expect("disable"), "a workspace with documents can be disabled");
    let first = get_workspace(&conn, &made.id).expect("get").expect("row").disabled_at;
    assert!(first.is_some());
    assert!(disable_workspace(&conn, &made.id).expect("again"));
    assert_eq!(get_workspace(&conn, &made.id).expect("get").expect("row").disabled_at, first, "kept");

    let ids = |rows: Vec<WorkspaceRow>| rows.into_iter().map(|w| w.id).collect::<Vec<_>>();
    assert!(!ids(list_workspaces(&conn).expect("list")).contains(&made.id));
    assert_eq!(ids(list_disabled_workspaces(&conn).expect("disabled")), vec![made.id.clone()]);
    assert!(ids(list_all_workspaces(&conn).expect("all")).contains(&made.id));
    assert_eq!(count(&conn, &made.doc_table), 1, "the document stays");
    assert!(create_workspace(&conn, "Temp", None).is_err(), "the name stays taken");
    assert_eq!(
        get_workspace_by_name(&conn, "Temp").expect("by name").map(|w| w.id),
        Some(made.id.clone())
    );

    assert!(enable_workspace(&conn, &made.id).expect("enable"));
    assert!(ids(list_workspaces(&conn).expect("list")).contains(&made.id));
    assert!(list_disabled_workspaces(&conn).expect("disabled").is_empty());
    assert!(!disable_workspace(&conn, "ghost").expect("missing"));
    assert!(!enable_workspace(&conn, "ghost").expect("missing"));
}

#[test]
fn the_disabled_at_column_is_empty_for_every_existing_row() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    for table in ["workspace", "project"] {
        assert!(columns(&conn, table).iter().any(|c| c.0 == "disabled_at"), "{table}");
        let disabled: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table} WHERE disabled_at IS NOT NULL"), [], |r| r.get(0))
            .expect("count");
        assert_eq!(disabled, 0, "{table}");
    }
}

#[test]
fn rename_changes_name_only_and_id_and_table_are_immutable() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Before", None).expect("create");
    assert!(rename_workspace(&conn, &made.id, "After").expect("rename"));
    assert!(rename_workspace(&conn, "default", "Home").expect("rename default"));
    let got = get_workspace(&conn, &made.id).expect("get").expect("row");
    assert_eq!((got.name.as_str(), got.id.as_str(), got.doc_table.as_str()), ("After", made.id.as_str(), made.doc_table.as_str()));
    assert!(rename_workspace(&conn, &made.id, "Home").is_err(), "name is unique");
    assert!(rename_workspace(&conn, &made.id, " ").is_err());
    assert!(!rename_workspace(&conn, "missing", "X").expect("missing"));

    assert!(conn.execute("UPDATE workspace SET id = 'other' WHERE id = ?1", [&made.id]).is_err());
    assert!(conn.execute("UPDATE workspace SET doc_table = 'doc_x' WHERE id = ?1", [&made.id]).is_err());
    assert!(set_workspace_view_mode(&conn, &made.id, ViewMode::Timeline).expect("mode"));
    assert_eq!(
        get_workspace(&conn, &made.id).expect("get").expect("row").view_mode,
        ViewMode::Timeline
    );
}

#[test]
fn registry_rejects_a_doc_table_that_does_not_match_the_id() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    assert!(conn
        .execute(
            "INSERT INTO workspace(id, name, doc_table) VALUES ('x1', 'X', 'note')",
            []
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO workspace(id, name, doc_table) VALUES ('X1', 'X', 'doc_X1')",
            []
        )
        .is_err());
}

#[test]
fn projects_are_unique_per_workspace_and_need_a_real_workspace() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    // A fresh database already carries the migrated `inbox` note category in `default`.
    let before = list_projects(&conn, "default").expect("list").len();
    insert_project(&conn, &ProjectRow::new("p1", "default", "Work", "")).expect("p1");
    insert_project(&conn, &ProjectRow::new("p2", "default", "Again", "")).expect("second project of the same workspace");
    insert_project(&conn, &ProjectRow::new("p3", "knowledge", "Work", "")).expect("same title elsewhere");
    assert!(
        insert_project(&conn, &ProjectRow::new("p4", "nope", "X", "")).is_err(),
        "FK to workspace"
    );

    let found = get_project(&conn, "p3").expect("get").expect("row");
    assert_eq!(found.id, "p3");
    let mut changed = found.clone();
    changed.title = "Renamed".into();
    changed.description = "分组：AI".into();
    assert!(update_project(&conn, &changed).expect("update"));
    assert_eq!(get_project(&conn, "p3").expect("get"), Some(changed));
    assert_eq!(list_projects(&conn, "default").expect("list").len(), before + 2);
    assert!(!update_project(&conn, &ProjectRow::new("none", "default", "F", "")).expect("missing"));
}

#[test]
fn doc_tables_need_a_real_project_and_block_a_raw_delete_of_it() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    insert_project(&conn, &ProjectRow::new("p1", "default", "Work", "")).expect("project");
    assert!(insert_doc(&conn, "doc_default", "d1", "missing").is_err(), "FK to project");
    insert_doc(&conn, "doc_default", "d1", "p1").expect("doc");
    assert!(insert_doc(&conn, "doc_default", "d1", "p1").is_err(), "primary key");
    assert!(
        conn.execute("DELETE FROM project WHERE id = 'p1'", []).is_err(),
        "a document still references it"
    );
    conn.execute("DELETE FROM doc_default WHERE doc_id = 'd1'", []).expect("remove doc");
    assert!(conn.execute("DELETE FROM project WHERE id = 'p1'", []).is_ok());
}

#[test]
fn disable_hides_a_project_keeps_it_by_id_and_enable_brings_it_back() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    insert_project(&conn, &ProjectRow::new("p1", "default", "Work", "")).expect("project");
    insert_doc(&conn, "doc_default", "d1", "p1").expect("doc");

    assert!(disable_project(&conn, "p1").expect("disable"), "a project with documents can be disabled");
    let ids = |rows: Vec<ProjectRow>| rows.into_iter().map(|p| p.id).collect::<Vec<_>>();
    assert!(!ids(list_projects(&conn, "default").expect("list")).contains(&"p1".to_string()));
    assert_eq!(ids(list_disabled_projects(&conn, "default").expect("disabled")), vec!["p1".to_string()]);
    assert!(ids(list_all_projects(&conn, "default").expect("all")).contains(&"p1".to_string()));
    let kept = get_project(&conn, "p1").expect("get").expect("row");
    assert!(kept.disabled_at.is_some(), "still found by id");

    assert!(enable_project(&conn, "p1").expect("enable"));
    assert!(ids(list_projects(&conn, "default").expect("list")).contains(&"p1".to_string()));
    assert!(!disable_project(&conn, "ghost").expect("missing"));
    assert!(!enable_project(&conn, "ghost").expect("missing"));
}

#[test]
fn icons_are_stored_per_workspace_and_project_and_can_be_cleared() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Iconic", None).expect("create");
    assert_eq!(made.icon, None, "new rows start without an icon");

    assert!(set_workspace_icon(&conn, &made.id, Some("03abcd")).expect("set"));
    assert_eq!(get_workspace(&conn, &made.id).expect("get").expect("row").icon.as_deref(), Some("03abcd"));
    assert!(set_workspace_icon(&conn, "default", Some("00zz09")).expect("default may have one"));
    assert!(set_workspace_icon(&conn, &made.id, None).expect("clear"));
    assert_eq!(get_workspace(&conn, &made.id).expect("get").expect("row").icon, None);
    assert!(!set_workspace_icon(&conn, "missing", Some("03abcd")).expect("missing"));

    let mut row = ProjectRow::new("pi", &made.id, "Work", "");
    row.icon = Some("01a1b2".into());
    insert_project(&conn, &row).expect("insert with icon");
    assert_eq!(get_project(&conn, "pi").expect("get").expect("row").icon.as_deref(), Some("01a1b2"));
    row.icon = None;
    assert!(update_project(&conn, &row).expect("update"));
    assert_eq!(get_project(&conn, "pi").expect("get").expect("row").icon, None);
}

fn workspace_ids(conn: &Connection) -> Vec<String> {
    list_workspaces(conn).expect("list").into_iter().map(|w| w.id).collect()
}

fn project_ids(conn: &Connection, workspace: &str) -> Vec<String> {
    list_projects(conn, workspace).expect("list").into_iter().map(|p| p.id).collect()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn new_workspaces_and_projects_are_appended_and_the_default_can_move() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let zeta = create_workspace(&conn, "Zeta", None).expect("zeta");
    let alpha = create_workspace(&conn, "Alpha", None).expect("alpha");
    assert_eq!(
        workspace_ids(&conn),
        vec!["default".to_string(), "knowledge".to_string(), zeta.id.clone(), alpha.id.clone()],
        "appended in creation order, not by name"
    );

    insert_project(&conn, &ProjectRow::new("pz", &zeta.id, "Zz", "")).expect("pz");
    insert_project(&conn, &ProjectRow::new("pa", &zeta.id, "Aa", "")).expect("pa");
    assert_eq!(project_ids(&conn, &zeta.id), strings(&["pz", "pa"]));

    let mut all = workspace_ids(&conn);
    all.rotate_left(1);
    reorder_workspaces(&conn, &all).expect("move default to the end");
    assert_eq!(workspace_ids(&conn), all);
    assert_eq!(workspace_ids(&conn).last().map(String::as_str), Some("default"));

    reorder_projects(&conn, &zeta.id, &strings(&["pa", "pz"])).expect("reorder projects");
    assert_eq!(project_ids(&conn, &zeta.id), strings(&["pa", "pz"]));
}

#[test]
fn a_reorder_must_name_every_entry_once_and_changes_nothing_otherwise() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Made", None).expect("made");
    let before = workspace_ids(&conn);

    for bad in [
        strings(&["default"]),
        strings(&["default", "knowledge", &made.id, &made.id]),
        strings(&["default", "knowledge", &made.id, "ghost"]),
        vec![],
    ] {
        assert!(reorder_workspaces(&conn, &bad).is_err(), "{bad:?}");
        assert_eq!(workspace_ids(&conn), before, "{bad:?} left the order alone");
    }

    insert_project(&conn, &ProjectRow::new("p1", &made.id, "One", "")).expect("p1");
    insert_project(&conn, &ProjectRow::new("p2", &made.id, "Two", "")).expect("p2");
    let projects = project_ids(&conn, &made.id);
    for bad in [strings(&["p1"]), strings(&["p1", "p1", "p2"]), strings(&["p1", "p2", "ghost"])] {
        assert!(reorder_projects(&conn, &made.id, &bad).is_err(), "{bad:?}");
        assert_eq!(project_ids(&conn, &made.id), projects);
    }
}

#[test]
fn disabled_entries_keep_their_place_and_come_back_to_it() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Made", None).expect("made");
    for (id, folder) in [("p1", "one"), ("p2", "two"), ("p3", "three")] {
        insert_project(&conn, &ProjectRow::new(id, &made.id, folder, "")).expect(folder);
    }
    assert!(disable_project(&conn, "p2").expect("disable"));
    assert_eq!(project_ids(&conn, &made.id), strings(&["p1", "p3"]));

    // The caller passes every project, the disabled one included, at the position it wants.
    reorder_projects(&conn, &made.id, &strings(&["p3", "p2", "p1"])).expect("reorder");
    assert_eq!(project_ids(&conn, &made.id), strings(&["p3", "p1"]));
    assert!(enable_project(&conn, "p2").expect("enable"));
    assert_eq!(project_ids(&conn, &made.id), strings(&["p3", "p2", "p1"]), "back in place");

    assert!(disable_workspace(&conn, &made.id).expect("disable workspace"));
    assert!(!workspace_ids(&conn).contains(&made.id));
    assert!(enable_workspace(&conn, &made.id).expect("enable workspace"));
    assert_eq!(workspace_ids(&conn).last(), Some(&made.id), "enabled workspace is where it was");
}
