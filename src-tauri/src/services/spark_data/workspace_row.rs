//! The `workspace` registry: one row per workspace, pointing at its document table.
//!
//! `id` and `doc_table` never change after creation (a trigger enforces it);
//! only `name` and `view_mode` are editable. The default workspace cannot be deleted.

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::doc_table::{create_doc_table, doc_table_for_id, validate_doc_table_name};
use crate::services::id::random_alnum6;

/// Id of the built-in default workspace (the former Notes).
pub const DEFAULT_WORKSPACE_ID: &str = "default";
/// Id of the seeded workspace that took over the former Knowledge.
pub const KNOWLEDGE_WORKSPACE_ID: &str = "knowledge";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Directory,
    Timeline,
}

impl ViewMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Directory => "directory",
            ViewMode::Timeline => "timeline",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "directory" => Ok(ViewMode::Directory),
            "timeline" => Ok(ViewMode::Timeline),
            other => Err(format!("unknown view_mode: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub id: String,
    pub name: String,
    pub doc_table: String,
    pub view_mode: ViewMode,
    pub is_default: bool,
    pub created_at: Option<String>,
}

const COLUMNS: &str = "id, name, doc_table, view_mode, is_default, created_at";

fn map_row(row: &Row<'_>) -> rusqlite::Result<WorkspaceRow> {
    let mode: String = row.get(3)?;
    Ok(WorkspaceRow {
        id: row.get(0)?,
        name: row.get(1)?,
        doc_table: row.get(2)?,
        view_mode: ViewMode::parse(&mode).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, e.into())
        })?,
        is_default: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
    })
}

fn clean_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("workspace name must not be empty".to_string());
    }
    Ok(name)
}

fn mint_id(conn: &Connection) -> Result<String, String> {
    for _ in 0..8 {
        let id = random_alnum6();
        if get_workspace(conn, &id)?.is_none() {
            return Ok(id);
        }
    }
    Err("could not mint a unique workspace id".to_string())
}

/// Register a workspace and create its document table in one transaction.
/// The caller supplies only a display name; `id` and table name are Host-made.
pub fn create_workspace(
    conn: &Connection,
    name: &str,
    view_mode: Option<ViewMode>,
) -> Result<WorkspaceRow, String> {
    let name = clean_name(name)?;
    let mode = view_mode.unwrap_or(ViewMode::Directory);
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let id = mint_id(&tx)?;
    let doc_table = doc_table_for_id(&id)?;
    tx.execute(
        &format!(
            "INSERT INTO workspace({COLUMNS})
             VALUES (?1, ?2, ?3, ?4, 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))"
        ),
        params![id, name, doc_table, mode.as_str()],
    )
    .map_err(|e| e.to_string())?;
    create_doc_table(&tx, &doc_table)?;
    let row = get_workspace(&tx, &id)?.ok_or("workspace row missing after insert")?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(row)
}

pub fn get_workspace(conn: &Connection, id: &str) -> Result<Option<WorkspaceRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM workspace WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Default workspace first, then name (case-insensitive).
pub fn list_workspaces(conn: &Connection) -> Result<Vec<WorkspaceRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM workspace ORDER BY is_default DESC, lower(name)"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// `false` when the workspace does not exist.
pub fn rename_workspace(conn: &Connection, id: &str, name: &str) -> Result<bool, String> {
    let name = clean_name(name)?;
    conn.execute("UPDATE workspace SET name = ?2 WHERE id = ?1", params![id, name])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}

/// `false` when the workspace does not exist.
pub fn set_workspace_view_mode(
    conn: &Connection,
    id: &str,
    view_mode: ViewMode,
) -> Result<bool, String> {
    conn.execute(
        "UPDATE workspace SET view_mode = ?2 WHERE id = ?1",
        params![id, view_mode.as_str()],
    )
    .map(|changed| changed > 0)
    .map_err(|e| e.to_string())
}

/// Delete an empty workspace: drops its document table and registry row together.
/// Refuses the default workspace and any workspace that still has projects.
/// `false` when it did not exist.
pub fn delete_workspace(conn: &Connection, id: &str) -> Result<bool, String> {
    let Some(row) = get_workspace(conn, id)? else {
        return Ok(false);
    };
    if row.is_default {
        return Err("the default workspace cannot be deleted".to_string());
    }
    let projects: i64 = conn
        .query_row("SELECT count(*) FROM project WHERE workspace_id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if projects > 0 {
        return Err(format!("workspace {id} still has {projects} project(s)"));
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    // `doc_table` came out of the registry, whose CHECK ties it to a valid id.
    validate_doc_table_name(&row.doc_table)?;
    tx.execute_batch(&format!("DROP TABLE {}", row.doc_table))
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM workspace WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}
