use super::*;

use rusqlite::Connection;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let path = dir.path().join("data").join(DB_FILE);
    (dir, path)
}

fn default_ws(conn: &Connection) -> WorkspaceRow {
    get_default_workspace(conn).expect("default workspace")
}

/// The seed has only `Default`; this makes a second, timeline workspace named `Knowledge`.
fn knowledge_ws(conn: &Connection) -> WorkspaceRow {
    match get_workspace_by_name(conn, "Knowledge").expect("lookup") {
        Some(row) => row,
        None => create_workspace(conn, "Knowledge", Some(ViewMode::Timeline)).expect("create Knowledge"),
    }
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

fn insert_doc(conn: &Connection, doc_id: &str, project_id: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO document(doc_id, raw_path, project_id) VALUES (?1, 'a.md', ?2)",
        [doc_id, project_id],
    )
}

#[test]
fn seed_registry_rows_and_modes_are_right() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let rows = list_workspaces(&conn).expect("list");
    assert_eq!(rows.len(), 1, "the seed is Default only");
    assert_eq!((rows[0].name.as_str(), rows[0].view_mode, rows[0].is_default), ("Default", ViewMode::Directory, true));
    assert_ne!(rows[0].id, "default");
    assert!(get_workspace_by_name(&conn, "Knowledge").expect("lookup").is_none());
}

#[test]
fn the_schema_has_one_document_table_and_no_per_workspace_tables() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    create_workspace(&conn, "Scratch", None).expect("create");
    knowledge_ws(&conn);
    let mut tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("tables")
        .collect::<Result<_, _>>()
        .expect("rows");
    tables.retain(|t| !t.starts_with("sqlite_"));
    assert_eq!(tables, ["document", "project", "tag", "workspace"]);
    let names: Vec<String> = columns(&conn, "document").into_iter().map(|c| c.0).collect();
    assert_eq!(names[..2], ["doc_id", "project_id"]);
    assert!(!names.contains(&"workspace_id".to_string()), "the workspace comes from the project");
    assert!(!columns(&conn, "workspace").iter().any(|c| c.0 == "doc_table"));
}

#[test]
fn create_workspace_registers_row_and_inbox_in_one_step() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "  Ai Memory  ", None).expect("create");
    assert_eq!(made.name, "Ai Memory");
    assert_eq!(made.view_mode, ViewMode::Directory);
    assert!(!made.is_default);
    assert!(validate_workspace_id(&made.id).is_ok(), "id {:?}", made.id);
    assert!(get_default_project(&conn, &made.id).expect("inbox").is_some());
    assert_eq!(get_workspace(&conn, &made.id).expect("get"), Some(made.clone()));

    let timeline = create_workspace(&conn, "Log", Some(ViewMode::Timeline)).expect("create");
    assert_eq!(timeline.view_mode, ViewMode::Timeline);
    assert_ne!(timeline.id, made.id);
}

#[test]
fn a_workspace_is_created_with_its_inbox_or_not_at_all() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Made", None).expect("made");
    let inbox = get_default_project(&conn, &made.id).expect("lookup").expect("inbox");
    assert!(inbox.is_default && inbox.name == "Inbox");

    // The inbox is the last step; when it fails the row must not stay.
    conn.execute_batch(
        "CREATE TRIGGER block_project BEFORE INSERT ON project
         BEGIN SELECT RAISE(ABORT, 'blocked by test'); END;",
    )
    .expect("trigger");
    let workspaces = count(&conn, "workspace");
    assert!(create_workspace(&conn, "Lost", None).is_err());
    assert_eq!(count(&conn, "workspace"), workspaces, "no registry row left behind");
    assert!(get_workspace_by_name(&conn, "Lost").expect("lookup").is_none());
}

#[test]
fn create_workspace_rejects_blank_and_duplicate_names_without_leaving_a_row() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    assert!(create_workspace(&conn, "   ", None).is_err());
    assert!(create_workspace(&conn, "Default", None).is_err(), "name is unique");
    assert_eq!(count(&conn, "workspace"), 1);
    assert_eq!(count(&conn, "project"), 1, "only the Default workspace's inbox");
}

#[test]
fn workspace_ids_are_lowercase_ascii_names_for_a_folder() {
    for ok in ["default", "a1_b2", "0", "zl7mly"] {
        assert!(validate_workspace_id(ok).is_ok(), "{ok}");
    }
    for bad in [
        "", "Ws", "a-b", "a b", "a;b", "a\"", "ä", "a.b", "../x", "a/b", "x".repeat(60).as_str(),
    ] {
        assert!(validate_workspace_id(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn default_workspace_cannot_be_disabled_and_not_deleted_even_by_raw_sql() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let default = default_ws(&conn);
    assert!(disable_workspace(&conn, &default.id).is_err());
    assert!(delete_empty_workspace(&conn, &default.id).is_err());
    assert!(conn.execute("DELETE FROM workspace WHERE id = ?1", [&default.id]).is_err(), "trigger");
    assert!(get_workspace(&conn, &default.id).expect("get").is_some());
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
fn delete_empty_workspace_drops_a_fresh_workspace_without_documents_and_refuses_one_with_documents() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Temp", None).expect("create");
    insert_project(&conn, &ProjectRow::new("p1", &made.id, "Notes", "")).expect("project");
    insert_doc(&conn, "d1", "p1").expect("doc");
    assert!(delete_empty_workspace(&conn, &made.id).is_err(), "has a document");
    assert!(get_project(&conn, "p1").expect("get").is_some());
    assert!(get_workspace(&conn, &made.id).expect("get").is_some(), "row stays when refused");

    conn.execute("DELETE FROM document WHERE doc_id = 'd1'", []).expect("remove doc");
    assert!(delete_empty_workspace(&conn, &made.id).expect("delete"), "the empty project goes with it");
    assert!(get_project(&conn, "p1").expect("get").is_none());
    assert!(get_workspace(&conn, &made.id).expect("get").is_none());
    assert!(!delete_empty_workspace(&conn, &made.id).expect("again"), "already gone");
}

#[test]
fn delete_empty_workspace_drops_every_project_and_frees_the_name() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Temp", None).expect("create");
    let inbox = get_default_project(&conn, &made.id).expect("lookup").expect("inbox");
    insert_project(&conn, &ProjectRow::new("p1", &made.id, "Notes", "")).expect("project");
    insert_project(&conn, &ProjectRow::new("p2", &made.id, "Work", "")).expect("project");
    assert_eq!(list_projects(&conn, &made.id).expect("list").len(), 3);

    assert!(delete_empty_workspace(&conn, &made.id).expect("delete"));
    assert!(get_project(&conn, &inbox.id).expect("get").is_none());
    assert!(get_project(&conn, "p1").expect("get").is_none());
    assert!(get_project(&conn, "p2").expect("get").is_none());
    assert!(get_workspace(&conn, &made.id).expect("get").is_none());
    assert!(list_projects(&conn, &made.id).expect("list").is_empty());

    let again = create_workspace(&conn, "Temp", None).expect("name is free after delete");
    assert_eq!(again.name, "Temp");
    assert_ne!(again.id, made.id);
}

#[test]
fn disable_hides_a_workspace_keeps_its_data_and_enable_brings_it_back() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Temp", None).expect("create");
    insert_project(&conn, &ProjectRow::new("p1", &made.id, "Notes", "")).expect("project");
    insert_doc(&conn, "d1", "p1").expect("doc");
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
    assert_eq!(count(&conn, "document"), 1, "the document stays");
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
fn rename_changes_name_only_and_id_is_immutable() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Before", None).expect("create");
    assert!(rename_workspace(&conn, &made.id, "After").expect("rename"));
    assert!(rename_workspace(&conn, &default_ws(&conn).id, "Home").expect("rename default"));
    let got = get_workspace(&conn, &made.id).expect("get").expect("row");
    assert_eq!((got.name.as_str(), got.id.as_str()), ("After", made.id.as_str()));
    assert!(rename_workspace(&conn, &made.id, "Home").is_err(), "name is unique");
    assert!(rename_workspace(&conn, &made.id, " ").is_err());
    assert!(!rename_workspace(&conn, "missing", "X").expect("missing"));

    assert!(conn.execute("UPDATE workspace SET id = 'other' WHERE id = ?1", [&made.id]).is_err());
    assert!(conn.execute("UPDATE workspace SET is_default = 1 WHERE id = ?1", [&made.id]).is_err());
    assert!(set_workspace_view_mode(&conn, &made.id, ViewMode::Timeline).expect("mode"));
    assert_eq!(
        get_workspace(&conn, &made.id).expect("get").expect("row").view_mode,
        ViewMode::Timeline
    );
}

#[test]
fn registry_rejects_an_id_that_is_not_a_lowercase_name() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    for bad in ["", "X1", "a-b", "a b"] {
        assert!(
            conn.execute("INSERT INTO workspace(id, name) VALUES (?1, 'X')", [bad]).is_err(),
            "{bad:?}"
        );
    }
}

#[test]
fn projects_are_unique_per_workspace_and_need_a_real_workspace() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    // A fresh database already carries the migrated `inbox` note category in Default.
    let default = default_ws(&conn);
    let knowledge = knowledge_ws(&conn);
    let before = list_projects(&conn, &default.id).expect("list").len();
    insert_project(&conn, &ProjectRow::new("p1", &default.id, "Work", "")).expect("p1");
    insert_project(&conn, &ProjectRow::new("p2", &default.id, "Again", "")).expect("second project of the same workspace");
    insert_project(&conn, &ProjectRow::new("p3", &knowledge.id, "Work", "")).expect("same title elsewhere");
    assert!(
        insert_project(&conn, &ProjectRow::new("p4", "nope", "X", "")).is_err(),
        "FK to workspace"
    );

    let found = get_project(&conn, "p3").expect("get").expect("row");
    assert_eq!(found.id, "p3");
    let mut changed = found.clone();
    changed.name = "Renamed".into();
    changed.description = "分组：AI".into();
    assert!(update_project(&conn, &changed).expect("update"));
    assert_eq!(get_project(&conn, "p3").expect("get"), Some(changed));
    assert_eq!(list_projects(&conn, &default.id).expect("list").len(), before + 2);
    assert!(!update_project(&conn, &ProjectRow::new("none", &default.id, "F", "")).expect("missing"));
}

#[test]
fn documents_need_a_real_project_and_block_a_raw_delete_of_it() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let default = default_ws(&conn);
    insert_project(&conn, &ProjectRow::new("p1", &default.id, "Work", "")).expect("project");
    assert!(insert_doc(&conn, "d1", "missing").is_err(), "FK to project");
    insert_doc(&conn, "d1", "p1").expect("doc");
    assert!(insert_doc(&conn, "d1", "p1").is_err(), "primary key");
    assert!(
        conn.execute("DELETE FROM project WHERE id = 'p1'", []).is_err(),
        "a document still references it"
    );
    conn.execute("DELETE FROM document WHERE doc_id = 'd1'", []).expect("remove doc");
    assert!(conn.execute("DELETE FROM project WHERE id = 'p1'", []).is_ok());
}

#[test]
fn a_legacy_disabled_at_does_not_hide_a_project() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let default = default_ws(&conn);
    insert_project(&conn, &ProjectRow::new("p1", &default.id, "Work", "")).expect("project");
    insert_doc(&conn, "d1", "p1").expect("doc");
    conn.execute("UPDATE project SET disabled_at = '2026-01-01T00:00:00Z' WHERE id = 'p1'", [])
        .expect("legacy mark");

    let ids = list_projects(&conn, &default.id).expect("list").into_iter().map(|p| p.id).collect::<Vec<_>>();
    assert!(ids.contains(&"p1".to_string()), "the old column is not read");
    assert!(get_project(&conn, "p1").expect("get").is_some());
}

#[test]
fn icons_are_stored_per_workspace_and_project_and_can_be_cleared() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Iconic", None).expect("create");
    assert_eq!(made.icon, None, "new rows start without an icon");

    assert!(set_workspace_icon(&conn, &made.id, Some("03abcd")).expect("set"));
    assert_eq!(get_workspace(&conn, &made.id).expect("get").expect("row").icon.as_deref(), Some("03abcd"));
    assert!(set_workspace_icon(&conn, &default_ws(&conn).id, Some("00zz09")).expect("default may have one"));
    assert!(set_workspace_icon(&conn, &made.id, None).expect("clear"));
    assert_eq!(get_workspace(&conn, &made.id).expect("get").expect("row").icon, None);
    assert!(!set_workspace_icon(&conn, "missing", Some("03abcd")).expect("missing"));

    assert_eq!(made.description, "", "new rows start with an empty description");
    assert!(set_workspace_description(&conn, &made.id, "Daily notes").expect("set"));
    assert_eq!(
        get_workspace(&conn, &made.id).expect("get").expect("row").description,
        "Daily notes"
    );
    assert!(set_workspace_description(&conn, &made.id, "").expect("clear"));
    assert_eq!(get_workspace(&conn, &made.id).expect("get").expect("row").description, "");
    assert!(!set_workspace_description(&conn, "missing", "x").expect("missing"));

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
    let default = default_ws(&conn);
    let knowledge = knowledge_ws(&conn);
    let zeta = create_workspace(&conn, "Zeta", None).expect("zeta");
    let alpha = create_workspace(&conn, "Alpha", None).expect("alpha");
    assert_eq!(
        workspace_ids(&conn),
        vec![default.id.clone(), knowledge.id.clone(), zeta.id.clone(), alpha.id.clone()],
        "appended in creation order, not by name"
    );

    let zeta_inbox = get_default_project(&conn, &zeta.id).expect("lookup").expect("a new workspace has an inbox").id;
    insert_project(&conn, &ProjectRow::new("pz", &zeta.id, "Zz", "")).expect("pz");
    insert_project(&conn, &ProjectRow::new("pa", &zeta.id, "Aa", "")).expect("pa");
    assert_eq!(project_ids(&conn, &zeta.id), strings(&[&zeta_inbox, "pz", "pa"]));

    let mut all = workspace_ids(&conn);
    all.rotate_left(1);
    reorder_workspaces(&conn, &all).expect("move default to the end");
    assert_eq!(workspace_ids(&conn), all);
    assert_eq!(workspace_ids(&conn).last(), Some(&default.id));

    reorder_projects(&conn, &zeta.id, &strings(&["pa", "pz", &zeta_inbox])).expect("reorder projects");
    assert_eq!(project_ids(&conn, &zeta.id), strings(&["pa", "pz", &zeta_inbox]));
}

#[test]
fn a_reorder_must_name_every_entry_once_and_changes_nothing_otherwise() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let default = default_ws(&conn);
    let knowledge = knowledge_ws(&conn);
    let made = create_workspace(&conn, "Made", None).expect("made");
    let before = workspace_ids(&conn);

    for bad in [
        vec![default.id.clone()],
        vec![default.id.clone(), knowledge.id.clone(), made.id.clone(), made.id.clone()],
        vec![default.id.clone(), knowledge.id.clone(), made.id.clone(), "ghost".into()],
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
fn projects_keep_the_order_they_were_given_and_a_disabled_workspace_comes_back_to_its_place() {
    let (_dir, path) = temp_db();
    let conn = open_at(&path).expect("open");
    let made = create_workspace(&conn, "Made", None).expect("made");
    let inbox = get_default_project(&conn, &made.id).expect("lookup").expect("a new workspace has an inbox").id;
    for (id, folder) in [("p1", "one"), ("p2", "two"), ("p3", "three")] {
        insert_project(&conn, &ProjectRow::new(id, &made.id, folder, "")).expect(folder);
    }
    assert_eq!(project_ids(&conn, &made.id), strings(&[&inbox, "p1", "p2", "p3"]));

    // The caller passes every project at the position it wants.
    reorder_projects(&conn, &made.id, &strings(&["p3", "p2", "p1", &inbox])).expect("reorder");
    assert_eq!(project_ids(&conn, &made.id), strings(&["p3", "p2", "p1", &inbox]));

    assert!(disable_workspace(&conn, &made.id).expect("disable workspace"));
    assert!(!workspace_ids(&conn).contains(&made.id));
    assert!(enable_workspace(&conn, &made.id).expect("enable workspace"));
    assert_eq!(workspace_ids(&conn).last(), Some(&made.id), "enabled workspace is where it was");
}
