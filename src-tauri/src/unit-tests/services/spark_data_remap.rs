//! Migration step 15: remap seeded `default` / `knowledge` ids.

use super::db::{open_with_steps, MIGRATIONS};
use super::prefix_legacy_doc_ids::PREFIX_LEGACY_DOC_IDS_STEP;
use super::remap_builtin_ids::REMAP_BUILTIN_WORKSPACE_IDS_STEP;
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

fn knowledge_ws(conn: &Connection) -> WorkspaceRow {
    get_workspace_by_name(conn, "Knowledge")
        .expect("lookup")
        .expect("Knowledge")
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0)).expect("version")
}

fn before_remap() -> &'static [&'static str] {
    let i = MIGRATIONS
        .iter()
        .position(|s| *s == REMAP_BUILTIN_WORKSPACE_IDS_STEP)
        .expect("remap step");
    &MIGRATIONS[..i]
}

#[test]
fn the_seeded_ids_stay_literal_until_the_remap_step() {
    let (_dir, path) = temp_db();
    let conn = open_with_steps(&path, before_remap()).expect("v15");
    assert_eq!(user_version(&conn), before_remap().len() as i64);
    assert!(get_workspace(&conn, "default").expect("get").is_some());
    assert!(get_workspace(&conn, "knowledge").expect("get").is_some());
}

#[test]
fn a_sidecar_map_is_reused_when_disk_already_has_the_new_folders() {
    let dir = tempfile::tempdir().expect("tmp");
    let data = dir.path().join("data");
    let path = data.join(DB_FILE);
    drop(open_with_steps(&path, before_remap()).expect("v15"));

    let new_default = "abc123";
    let new_knowledge = "xyz789";
    std::fs::create_dir_all(data.join("workspaces").join(new_default)).expect("dir");
    std::fs::write(data.join("workspaces").join(new_default).join("keep.txt"), "x").expect("file");
    std::fs::write(
        data.join(".remap-builtin-workspace-ids"),
        format!("default={new_default}\nknowledge={new_knowledge}\n"),
    )
    .expect("map");

    let conn = open_at(&path).expect("remap");
    assert_eq!(default_ws(&conn).id, new_default);
    assert_eq!(knowledge_ws(&conn).id, new_knowledge);
    assert_eq!(default_ws(&conn).doc_table, format!("doc_{new_default}"));
    assert_eq!(knowledge_ws(&conn).doc_table, format!("doc_{new_knowledge}"));
    assert!(data.join("workspaces").join(new_default).join("keep.txt").is_file());
    assert!(!data.join("workspaces").join("default").exists());
    assert!(!data.join(".remap-builtin-workspace-ids").exists());
}

const PREFIXED_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PLAIN_HEX: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn prefixed_doc_ids_are_rewritten_and_old_links_still_resolve() {
    let (_dir, path) = temp_db();
    let before = open_with_steps(&path, before_remap()).expect("v15");
    insert_doc(
        &before,
        "doc_default",
        &DocRow::new(&format!("default-{PREFIXED_HEX}"), "a.md", "inbox"),
    )
    .expect("prefixed default");
    insert_doc(&before, "doc_default", &DocRow::new(PLAIN_HEX, "b.md", "inbox")).expect("plain");
    let k_project = ProjectRow::new("k1", "knowledge", "Demo", "");
    insert_project(&before, &k_project).expect("knowledge project");
    insert_doc(
        &before,
        "doc_knowledge",
        &DocRow::new(&format!("knowledge-{PREFIXED_HEX}"), "c.md", "k1"),
    )
    .expect("prefixed knowledge");
    drop(before);

    let through_remap = MIGRATIONS
        .iter()
        .position(|s| *s == PREFIX_LEGACY_DOC_IDS_STEP)
        .expect("prefix step");
    let conn = open_with_steps(&path, &MIGRATIONS[..through_remap]).expect("remap");
    let default = default_ws(&conn);
    let knowledge = knowledge_ws(&conn);
    let new_default = format!("{}-{PREFIXED_HEX}", default.id);
    let new_knowledge = format!("{}-{PREFIXED_HEX}", knowledge.id);

    assert!(get_doc(&conn, &default.doc_table, &new_default).expect("get").is_some());
    assert!(get_doc(&conn, &default.doc_table, &format!("default-{PREFIXED_HEX}")).expect("get").is_none());
    assert!(get_doc(&conn, &default.doc_table, PLAIN_HEX).expect("get").is_some(), "plain hex stays");
    assert!(get_doc(&conn, &knowledge.doc_table, &new_knowledge).expect("get").is_some());

    let via_old = find_doc(&conn, &format!("default-{PREFIXED_HEX}")).expect("find").expect("row");
    assert_eq!(via_old.doc.doc_id, new_default);
    assert_eq!(via_old.workspace.id, default.id);
    let via_knowledge = find_doc(&conn, &format!("knowledge-{PREFIXED_HEX}")).expect("find").expect("row");
    assert_eq!(via_knowledge.doc.doc_id, new_knowledge);
    assert_eq!(find_doc(&conn, PLAIN_HEX).expect("find").expect("plain").doc.doc_id, PLAIN_HEX);
}
